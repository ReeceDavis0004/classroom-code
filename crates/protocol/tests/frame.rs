use protocol::{FrameError, FromRunner, MAX_FRAME, ToRunner, decode, encode};

fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    msg: T,
) {
    let frame = encode(&msg).unwrap();
    assert_eq!(
        u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize,
        frame.len() - 4
    );
    let (back, used) = decode::<T>(&frame).unwrap().unwrap();
    assert_eq!(back, msg);
    assert_eq!(used, frame.len());
}

#[test]
fn every_message_round_trips() {
    round_trip(ToRunner::PutFile {
        path: "main.c".into(),
        bytes: b"int main(){}".to_vec(),
    });
    round_trip(ToRunner::Exec {
        argv: vec!["./main".into()],
        stdin: b"1 2\n".to_vec(),
        cpu_limit_ms: 2000,
        wall_limit_ms: 4000,
        stdout_cap: 65536,
        stderr_cap: 4096,
    });
    round_trip(ToRunner::Stat {
        path: "Main.class".into(),
    });
    round_trip(ToRunner::Shutdown);
    round_trip(FromRunner::Ready { runner_version: 1 });
    round_trip(FromRunner::FileOk);
    round_trip(FromRunner::Stat {
        exists: true,
        size: 12,
    });
    round_trip(FromRunner::Exited {
        exit_code: None,
        signal: Some(9),
        cpu_ms: 2001,
        wall_ms: 2003,
        max_rss_kb: 1024,
        oom_kills: 0,
        timed_out: true,
        stdout: b"3\n".to_vec(),
        stdout_truncated: false,
        stderr: vec![],
        stderr_truncated: true,
    });
    round_trip(FromRunner::Error {
        message: "bad path".into(),
    });
}

#[test]
fn back_to_back_frames_decode_in_order() {
    let mut buf = encode(&FromRunner::FileOk).unwrap();
    buf.extend(encode(&FromRunner::Ready { runner_version: 7 }).unwrap());
    let (first, used) = decode::<FromRunner>(&buf).unwrap().unwrap();
    assert_eq!(first, FromRunner::FileOk);
    let (second, _) = decode::<FromRunner>(&buf[used..]).unwrap().unwrap();
    assert_eq!(second, FromRunner::Ready { runner_version: 7 });
}

#[test]
fn truncated_frame_waits_for_more() {
    let frame = encode(&ToRunner::Stat {
        path: "main.c".into(),
    })
    .unwrap();
    for cut in 0..frame.len() {
        assert!(
            decode::<ToRunner>(&frame[..cut]).unwrap().is_none(),
            "cut {cut}"
        );
    }
}

#[test]
fn encode_rejects_body_over_cap() {
    let msg = ToRunner::PutFile {
        path: "big".into(),
        bytes: vec![0; MAX_FRAME],
    };
    assert!(matches!(encode(&msg), Err(FrameError::TooLarge(_))));
}

#[test]
fn encode_accepts_body_at_cap() {
    let overhead = encode(&ToRunner::PutFile {
        path: "f".into(),
        bytes: vec![],
    })
    .unwrap()
    .len()
        - 4;
    let bytes = vec![0; MAX_FRAME - overhead - 3];
    let frame = encode(&ToRunner::PutFile {
        path: "f".into(),
        bytes,
    })
    .unwrap();
    assert_eq!(frame.len() - 4, MAX_FRAME);
    assert!(decode::<ToRunner>(&frame).unwrap().is_some());
}

#[test]
fn decode_rejects_oversized_header_before_body() {
    let header = ((MAX_FRAME + 1) as u32).to_be_bytes();
    assert!(matches!(
        decode::<ToRunner>(&header),
        Err(FrameError::TooLarge(n)) if n == MAX_FRAME + 1
    ));
}

#[test]
fn decode_rejects_garbage_body() {
    let mut frame = 2u32.to_be_bytes().to_vec();
    frame.extend([0xff, 0xff]);
    assert!(matches!(
        decode::<ToRunner>(&frame),
        Err(FrameError::Malformed(_))
    ));
}
