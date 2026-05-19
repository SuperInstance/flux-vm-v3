use flux_vm_v3::streaming::StreamState;
use flux_vm_v3::error::FluxError;

#[test]
fn test_stream_open_close() {
    let mut stream = StreamState::new();
    assert!(!stream.is_open());
    stream.open(64).unwrap();
    assert!(stream.is_open());
    stream.close().unwrap();
    assert!(!stream.is_open());
}

#[test]
fn test_stream_double_open() {
    let mut stream = StreamState::new();
    stream.open(64).unwrap();
    assert!(matches!(stream.open(32), Err(FluxError::StreamAlreadyOpen)));
}

#[test]
fn test_stream_push_without_open() {
    let mut stream = StreamState::new();
    assert!(matches!(stream.push(42), Err(FluxError::StreamNotOpen)));
}

#[test]
fn test_stream_check_range() {
    let mut stream = StreamState::new();
    stream.open(64).unwrap();
    for v in &[10, 20, 30, 40, 50, 60, 70, 80] {
        stream.push(*v).unwrap();
    }
    let pass = stream.check_range(0, 50);
    assert_eq!(pass, 5); // 10,20,30,40,50 pass
    assert_eq!(stream.results().len(), 8);
}

#[test]
fn test_stream_close_without_open() {
    let mut stream = StreamState::new();
    assert!(matches!(stream.close(), Err(FluxError::StreamNotOpen)));
}
