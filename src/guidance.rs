use bliss_playlist_guidance_spi::{
    Diagnostics, GuidanceHostRequestV1, GuidanceHostResponseV1, GuidanceRequest,
    GuidanceResponse, GuidanceSignal, SelectionTraceCandidate, SelectionTraceContribution,
    SelectionTraceV1, SPI_VERSION,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

const HOST_NAME: &str = "bliss-mixer";

pub fn score(request: GuidanceHostRequestV1) -> GuidanceHostResponseV1 {
    match run_provider_session(&request) {
        Ok((signals, diagnostics)) => GuidanceHostResponseV1 {
            response_version: "guidance_host_response_v1".into(),
            valid: true,
            selection_trace: Some(selection_trace(&request, &signals)),
            signals,
            diagnostics,
            diagnostic: String::new(),
        },
        Err(diagnostic) => GuidanceHostResponseV1 {
            response_version: "guidance_host_response_v1".into(),
            valid: false,
            signals: vec![],
            diagnostics: Diagnostics::default(),
            selection_trace: None,
            diagnostic,
        },
    }
}

fn run_provider_session(
    request: &GuidanceHostRequestV1,
) -> Result<(Vec<GuidanceSignal>, Diagnostics), String> {
    if request.request_version != "guidance_host_request_v1" {
        return Err("unsupported guidance host request version".into());
    }
    if request.provider.program.is_empty() {
        return Err("provider program is missing".into());
    }
    let mut command = Command::new(&request.provider.program);
    command
        .args(&request.provider.argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| format!("provider could not be started: {error}"))?;
    let stdin = child.stdin.take().ok_or("provider stdin is unavailable")?;
    let stdout = child.stdout.take().ok_or("provider stdout is unavailable")?;
    let child = Arc::new(Mutex::new(child));
    let worker_child = Arc::clone(&child);
    let worker_request = request.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = exchange_session(stdin, stdout, &worker_request);
        if let Ok(mut child) = worker_child.lock() {
            let _ = child.wait();
        }
        let _ = sender.send(result);
    });
    let timeout = Duration::from_millis(request.deadline_ms.max(1));
    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            if let Ok(mut child) = child.lock() {
                let _ = child.kill();
            }
            Err("provider session exceeded its deadline".into())
        }
        Err(error) => Err(format!("provider session failed: {error}")),
    }
}

fn exchange_session(
    mut stdin: impl Write,
    stdout: impl std::io::Read,
    request: &GuidanceHostRequestV1,
) -> Result<(Vec<GuidanceSignal>, Diagnostics), String> {
    let mut stdout = BufReader::new(stdout);
    let manifest = exchange(
        &mut stdin,
        &mut stdout,
        &GuidanceRequest::Describe {
            spi_version: SPI_VERSION,
        },
    )?;
    match manifest {
        GuidanceResponse::Manifest(manifest)
            if manifest.provider_id == request.provider.provider_id
                && manifest.spi_version == SPI_VERSION => {}
        GuidanceResponse::Manifest(_) => return Err("provider manifest does not match configuration".into()),
        _ => return Err("provider did not return a manifest".into()),
    }
    expect_provider(
        exchange(
            &mut stdin,
            &mut stdout,
            &GuidanceRequest::Prepare {
                spi_version: SPI_VERSION,
                job_id: request.job_id.clone(),
                options: request.provider.options.clone(),
                artifacts: request.provider.artifacts.clone(),
                resources: request.provider.resources.clone(),
                anchors: vec![],
            },
        )?,
        "prepared",
        &request.provider.provider_id,
    )?;
    let scores = exchange(
        &mut stdin,
        &mut stdout,
        &GuidanceRequest::Score {
            spi_version: SPI_VERSION,
            request_id: request.request_id.clone(),
            context: request.context.clone(),
            candidates: request.candidates.clone(),
        },
    )?;
    let (signals, diagnostics) = match scores {
        GuidanceResponse::Scores {
            provider_id,
            request_id,
            signals,
            diagnostics,
        } if provider_id == request.provider.provider_id && request_id == request.request_id => {
            (signals, diagnostics)
        }
        _ => return Err("provider did not return matching scores".into()),
    };
    expect_provider(
        exchange(
            &mut stdin,
            &mut stdout,
            &GuidanceRequest::Close {
                spi_version: SPI_VERSION,
            },
        )?,
        "closed",
        &request.provider.provider_id,
    )?;
    Ok((signals, diagnostics))
}

fn exchange(
    stdin: &mut impl Write,
    stdout: &mut impl BufRead,
    request: &GuidanceRequest,
) -> Result<GuidanceResponse, String> {
    let line = bliss_playlist_guidance_spi::encode(request)
        .map_err(|error| format!("cannot encode provider request: {error}"))?;
    writeln!(stdin, "{line}").map_err(|error| format!("provider input write failed: {error}"))?;
    stdin.flush().map_err(|error| format!("provider input flush failed: {error}"))?;
    let mut line = String::new();
    stdout
        .read_line(&mut line)
        .map_err(|error| format!("provider output read failed: {error}"))?;
    if line.is_empty() {
        return Err("provider ended without a response".into());
    }
    let response = bliss_playlist_guidance_spi::decode_response(&line)
        .map_err(|error| format!("provider returned invalid JSON: {error}; line={line:?}"))?;
    if let GuidanceResponse::Error { code, message, .. } = &response {
        return Err(format!("provider error {code}: {message}"));
    }
    Ok(response)
}

fn expect_provider(response: GuidanceResponse, expected: &str, provider_id: &str) -> Result<(), String> {
    let matches = match response {
        GuidanceResponse::Prepared { provider_id: actual, .. } if expected == "prepared" => actual == provider_id,
        GuidanceResponse::Closed { provider_id: actual } if expected == "closed" => actual == provider_id,
        _ => false,
    };
    matches
        .then_some(())
        .ok_or_else(|| format!("provider did not return matching {expected}"))
}

pub fn selection_trace(
    request: &GuidanceHostRequestV1,
    signals: &[GuidanceSignal],
) -> SelectionTraceV1 {
    let mut by_candidate = BTreeMap::<&str, Vec<&GuidanceSignal>>::new();
    for signal in signals {
        by_candidate
            .entry(signal.candidate_id.as_str())
            .or_default()
            .push(signal);
    }
    let candidates = request
        .candidates
        .iter()
        .map(|candidate| SelectionTraceCandidate {
            candidate_id: candidate.candidate_id.clone(),
            bliss_similarity: None,
            guidance: by_candidate
                .remove(candidate.candidate_id.as_str())
                .unwrap_or_default()
                .into_iter()
                .map(|signal| SelectionTraceContribution {
                    channel: signal.channel.clone(),
                    score: signal.score.clamp(-1.0, 1.0),
                    confidence: signal.confidence.clamp(0.0, 1.0),
                    contribution: contribution(&request.policy, signal),
                    observation: signal.observation.clone(),
                })
                .collect(),
            final_score: None,
            dominant_boost: None,
            stochastic_key: None,
            cutoff: None,
        })
        .collect();
    SelectionTraceV1 {
        trace_version: "selection_trace_v1".into(),
        provider_id: request.provider.provider_id.clone(),
        host: HOST_NAME.into(),
        policy: request.policy.clone(),
        candidates,
    }
}

fn contribution(policy: &Value, signal: &GuidanceSignal) -> f64 {
    let key = match signal.channel.as_str() {
        "playcount" => "playcount_influence",
        "last_played" => "last_played_influence",
        "library_age" => "library_age_influence",
        _ => return 0.0,
    };
    let influence = policy
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(-100.0, 100.0);
    (influence / 100.0) * signal.score.clamp(-1.0, 1.0) * signal.confidence.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bliss_playlist_guidance_spi::{
        Candidate, GuidanceHostRequestV1, GuidanceProviderConfig, GuidanceScope, GuidanceSignal,
        ScoreContext,
    };
    use serde_json::json;

    #[cfg(windows)]
    fn fixture_provider_command() -> (String, Vec<String>) {
        let manifest = r#"{"type":"manifest","spi_version":2,"provider_id":"library-signals-guidance","provider_version":"fixture","protocol":"bliss-guidance-jsonl-v2","capabilities":["global_candidate_guidance"],"channels":[]}"#;
        let prepared = r#"{"type":"prepared","provider_id":"library-signals-guidance","snapshot_id":null,"diagnostics":{}}"#;
        let scores = r#"{"type":"scores","provider_id":"library-signals-guidance","request_id":"dstm-candidate-pool","signals":[{"candidate_id":"file:///music/example.flac","channel":"playcount","scope":"global","score":-0.5,"confidence":1.0,"observation":{"playcount":2}}],"diagnostics":{}}"#;
        let closed = r#"{"type":"closed","provider_id":"library-signals-guidance"}"#;
        (
            "powershell.exe".into(),
            vec!["-NoProfile".into(), "-Command".into(), format!(
                "[Console]::WriteLine('{manifest}'); [Console]::WriteLine('{prepared}'); [Console]::WriteLine('{scores}'); [Console]::WriteLine('{closed}')",
            )],
        )
    }

    #[cfg(not(windows))]
    fn fixture_provider_command() -> (String, Vec<String>) {
        let transcript = concat!(
            "{\"type\":\"manifest\",\"spi_version\":2,\"provider_id\":\"library-signals-guidance\",\"provider_version\":\"fixture\",\"protocol\":\"bliss-guidance-jsonl-v2\",\"capabilities\":[\"global_candidate_guidance\"],\"channels\":[]}\n",
            "{\"type\":\"prepared\",\"provider_id\":\"library-signals-guidance\",\"snapshot_id\":null,\"diagnostics\":{}}\n",
            "{\"type\":\"scores\",\"provider_id\":\"library-signals-guidance\",\"request_id\":\"dstm-candidate-pool\",\"signals\":[{\"candidate_id\":\"file:///music/example.flac\",\"channel\":\"playcount\",\"scope\":\"global\",\"score\":-0.5,\"confidence\":1.0,\"observation\":{\"playcount\":2}}],\"diagnostics\":{}}\n",
            "{\"type\":\"closed\",\"provider_id\":\"library-signals-guidance\"}\n"
        );
        let mut responses = transcript.lines();
        let manifest = responses.next().unwrap();
        let prepared = responses.next().unwrap();
        let scores = responses.next().unwrap();
        let closed = responses.next().unwrap();
        (
            "sh".into(),
            vec![
                "-c".into(),
                format!(
                    "read _; printf '%s\\n' '{manifest}'; \
                     read _; printf '%s\\n' '{prepared}'; \
                     read _; printf '%s\\n' '{scores}'; \
                     read _; printf '%s\\n' '{closed}'"
                ),
            ],
        )
    }

    #[test]
    fn selection_trace_preserves_library_signal_observations_and_bounded_contributions() {
        let request = GuidanceHostRequestV1 {
            request_version: "guidance_host_request_v1".into(),
            job_id: "mix-42".into(),
            request_id: "dstm-candidate-pool".into(),
            deadline_ms: 500,
            provider: GuidanceProviderConfig {
                provider_id: "library-signals-guidance".into(),
                program: "/trusted/bliss-guidance-library-signals".into(),
                argv: vec![],
                options: json!({}),
                artifacts: vec![],
                resources: vec![],
            },
            policy: json!({ "playcount_influence": -80 }),
            context: ScoreContext {
                scope: GuidanceScope::Global,
                left_anchor_id: None,
                right_anchor_id: None,
                context_track_ids: vec![],
            },
            candidates: vec![Candidate {
                candidate_id: "file:///music/example.flac".into(),
                lms_urlmd5: Some("aabbcc".into()),
                database_file: None,
                title: None,
                artist: None,
                album: None,
                recording_mbid: None,
                artist_mbids: vec![],
            }],
        };
        let signals = vec![GuidanceSignal {
            candidate_id: "file:///music/example.flac".into(),
            channel: "playcount".into(),
            scope: GuidanceScope::Global,
            score: -0.5,
            confidence: 1.0,
            rationale: None,
            observed_at: None,
            observation: Some(json!({ "playcount": 2 })),
        }];

        let trace = selection_trace(&request, &signals);

        assert_eq!(trace.trace_version, "selection_trace_v1");
        assert_eq!(trace.candidates[0].guidance[0].observation, Some(json!({ "playcount": 2 })));
        assert!((trace.candidates[0].guidance[0].contribution - 0.4).abs() < 1e-12);
        assert!(trace.candidates[0].final_score.is_none());
    }

    #[test]
    fn native_host_runs_a_jsonl_provider_and_returns_library_signal_trace() {
        let (program, argv) = fixture_provider_command();
        let response = score(GuidanceHostRequestV1 {
            request_version: "guidance_host_request_v1".into(),
            job_id: "mix-42".into(),
            request_id: "dstm-candidate-pool".into(),
            deadline_ms: 500,
            provider: GuidanceProviderConfig {
                provider_id: "library-signals-guidance".into(),
                program,
                argv,
                options: json!({}),
                artifacts: vec![],
                resources: vec![],
            },
            policy: json!({ "playcount_influence": -80 }),
            context: ScoreContext {
                scope: GuidanceScope::Global,
                left_anchor_id: None,
                right_anchor_id: None,
                context_track_ids: vec![],
            },
            candidates: vec![Candidate {
                candidate_id: "file:///music/example.flac".into(),
                lms_urlmd5: Some("aabbcc".into()),
                database_file: None,
                title: None,
                artist: None,
                album: None,
                recording_mbid: None,
                artist_mbids: vec![],
            }],
        });

        assert!(response.valid, "{}", response.diagnostic);
        assert_eq!(response.signals.len(), 1);
        assert_eq!(response.selection_trace.unwrap().candidates[0].guidance[0].channel, "playcount");
    }
}
