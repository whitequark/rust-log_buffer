extern crate log_buffer;

use std::fmt::Write;
use log_buffer::LogBuffer;

#[test]
fn basic() {
    let mut storage = [0; 16];
    let mut buffer = LogBuffer::new(&mut storage);

    assert_eq!(buffer.is_empty(), true);
    assert_eq!(buffer.extract(), "");
    assert_eq!(buffer.is_empty(), true);

    write!(buffer, "foo").unwrap();
    assert_eq!(buffer.is_empty(), false);
    assert_eq!(buffer.extract(), "foo");
    assert_eq!(buffer.is_empty(), false);

    write!(buffer, "bar").unwrap();
    assert_eq!(buffer.extract(), "foobar");

    write!(buffer, "verylongthing").unwrap();
    assert_eq!(buffer.is_empty(), false);
    assert_eq!(buffer.extract(), "barverylongthing");
    assert_eq!(buffer.is_empty(), false);

    buffer.clear();
    assert_eq!(buffer.is_empty(), true);
}

#[test]
fn exhaustive() {
    for i in 0..17 {
        let mut storage = [0; 16];
        let mut buffer = LogBuffer::new(&mut storage);

        for _ in 0..i { write!(buffer, "x").unwrap() }
        write!(buffer, "abcdefghijklmnop").unwrap();
        assert_eq!(buffer.extract(), "abcdefghijklmnop");
    }
}

#[test]
fn cut_off_utf8() {
    let mut storage = [0; 16];
    let mut buffer = LogBuffer::new(&mut storage);

    // two code units
    write!(buffer, "ййййййййa").unwrap();
    assert_eq!(buffer.extract(), "йййййййa");

    // three code units
    write!(buffer, "ああああああa").unwrap();
    assert_eq!(buffer.extract(), "あああああa");

    // four code units
    write!(buffer, "😊😊😊😊a").unwrap();
    assert_eq!(buffer.extract(), "😊😊😊a");
}

#[test]
fn lines() {
    let mut storage = [0; 16];
    let mut buffer = LogBuffer::new(&mut storage);

    assert_eq!(buffer.extract(), "");

    write!(buffer, "\n1,hoge\n").unwrap();
    assert_eq!(buffer.extract_lines().collect::<Vec<&str>>(),
               vec!["1,hoge"]);

    write!(buffer, "2,fuga\n").unwrap();
    write!(buffer, "3,piyo\n").unwrap();
    assert_eq!(buffer.extract_lines().collect::<Vec<_>>(),
               vec!["2,fuga", "3,piyo"]);
}
