use crate::StoreError;

pub fn decode_aws_chunked(input: &[u8]) -> Result<Vec<u8>, StoreError> {
    let err = || StoreError::InvalidChunkedEncoding;
    let mut pos = 0;
    let mut output = Vec::new();

    loop {
        let line_end = memchr_crlf(input, pos).ok_or_else(err)?;
        let line = &input[pos..line_end];

        let hex_part = match line.iter().position(|&b| b == b';') {
            Some(i) => &line[..i],
            None => line,
        };

        let size = usize::from_str_radix(std::str::from_utf8(hex_part).map_err(|_| err())?, 16)
            .map_err(|_| err())?;

        pos = line_end + 2;

        if size == 0 {
            break;
        }

        let chunk_end = pos.checked_add(size).ok_or_else(err)?;
        if input.len() < chunk_end + 2 {
            return Err(err());
        }

        output.extend_from_slice(&input[pos..chunk_end]);
        pos = chunk_end + 2;
    }

    Ok(output)
}

fn memchr_crlf(haystack: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < haystack.len() {
        if haystack[i] == b'\r' && haystack[i + 1] == b'\n' {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunked_frame(chunks: &[&[u8]]) -> Vec<u8> {
        let mut buf = Vec::new();
        for chunk in chunks {
            buf.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            buf.extend_from_slice(chunk);
            buf.extend_from_slice(b"\r\n");
        }
        buf.extend_from_slice(b"0\r\n\r\n");
        buf
    }

    fn chunked_frame_with_signatures(chunks: &[&[u8]]) -> Vec<u8> {
        let mut buf = Vec::new();
        for chunk in chunks {
            buf.extend_from_slice(format!("{:x};chunk-signature=aaaa\r\n", chunk.len()).as_bytes());
            buf.extend_from_slice(chunk);
            buf.extend_from_slice(b"\r\n");
        }
        buf.extend_from_slice(b"0;chunk-signature=aaaa\r\n\r\n");
        buf
    }

    #[test]
    fn single_chunk() {
        let input = chunked_frame(&[b"hello world"]);
        let decoded = decode_aws_chunked(&input).unwrap();
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn multiple_chunks() {
        let input = chunked_frame(&[b"hello ", b"world"]);
        let decoded = decode_aws_chunked(&input).unwrap();
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn with_chunk_signature_extensions() {
        let input = chunked_frame_with_signatures(&[b"payload"]);
        let decoded = decode_aws_chunked(&input).unwrap();
        assert_eq!(decoded, b"payload");
    }

    #[test]
    fn empty_body() {
        let input = b"0\r\n\r\n";
        let decoded = decode_aws_chunked(input).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn binary_data() {
        let binary: Vec<u8> = (0..=255).collect();
        let input = chunked_frame(&[&binary]);
        let decoded = decode_aws_chunked(&input).unwrap();
        assert_eq!(decoded, binary);
    }

    #[test]
    fn large_multi_chunk() {
        let a = vec![0xAA; 16384];
        let b = vec![0xBB; 8192];
        let input = chunked_frame(&[&a, &b]);
        let decoded = decode_aws_chunked(&input).unwrap();
        let mut expected = a;
        expected.extend_from_slice(&b);
        assert_eq!(decoded, expected);
    }

    #[test]
    fn trailing_headers_ignored() {
        let mut input = Vec::new();
        input.extend_from_slice(b"5\r\nhello\r\n");
        input.extend_from_slice(b"0\r\n");
        input.extend_from_slice(b"x-amz-checksum:abc\r\n");
        input.extend_from_slice(b"\r\n");
        let decoded = decode_aws_chunked(&input).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn invalid_hex_returns_error() {
        let input = b"ZZZZ\r\ndata\r\n0\r\n\r\n";
        assert!(decode_aws_chunked(input).is_err());
    }

    #[test]
    fn truncated_input_returns_error() {
        let input = b"5\r\nhi";
        assert!(decode_aws_chunked(input).is_err());
    }
}
