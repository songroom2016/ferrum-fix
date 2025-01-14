use crate::buffer::Buffer;
use crate::tagvalue::{DecodeError, EncodeError};
use std::convert::TryInto;

// A tag-value message can't possibly be shorter than this.
//
//   8=?|9=?|35=?|10=???|
pub const MIN_FIX_MESSAGE_LEN_IN_BYTES: usize = 20;

/// The checksum field is composed of:
///  - `10=`       (3 characters)
///  - `XYZ`       (checksum value, always 3 characters)
///  - separator   (1 character)
/// Total: 7 characters.
pub const FIELD_CHECKSUM_LEN_IN_BYTES: usize = 7;

/// Parses an `u8` from `digits` and returns the result.
///
/// No error detection is performed. Values less than 100 must be zero-padded.
pub fn parse_u8_from_decimal(digits: [u8; 3]) -> u8 {
    digits[0]
        .wrapping_sub(b'0')
        .wrapping_mul(100)
        .wrapping_add(digits[1].wrapping_sub(b'0').wrapping_mul(10))
        .wrapping_add(digits[2].wrapping_sub(b'0'))
}

/// Returns the `CheckSum <10>` value of `data`.
///
/// # Examples
///
/// ```
/// use fefix::tagvalue::checksum_10;
///
/// assert_eq!(checksum_10(b""), 0x0);
/// assert_eq!(checksum_10(b"hunter2"), 0xc8);
/// ```
pub fn checksum_10(data: &[u8]) -> u8 {
    let mut value = 0u8;
    for byte in data {
        value = value.wrapping_add(*byte);
    }
    value
}

/// Returns a copy of the `CheckSum <10>` digits of `message`.
pub fn checksum_digits(message: &[u8]) -> [u8; 3] {
    debug_assert!(message.len() >= MIN_FIX_MESSAGE_LEN_IN_BYTES);
    message[message.len() - 4..message.len() - 1]
        .try_into()
        .unwrap()
}

pub fn verify_checksum(message: &[u8]) -> Result<(), DecodeError> {
    let nominal_checksum = parse_u8_from_decimal(checksum_digits(message));
    let actual_checksum = checksum_10(&message[..message.len() - FIELD_CHECKSUM_LEN_IN_BYTES]);
    if nominal_checksum != actual_checksum {
        Err(DecodeError::CheckSum)
    } else {
        Ok(())
    }
}

/// Verifies the `BodyLength(9)` field of the FIX message in `data`.
pub fn verify_body_length(
    data: &[u8],
    start_of_body: usize,
    nominal_body_length: usize,
) -> Result<(), DecodeError> {
    let body_length = data
        .len()
        .wrapping_sub(FIELD_CHECKSUM_LEN_IN_BYTES)
        .wrapping_sub(start_of_body);
    let end_of_body = data.len() - FIELD_CHECKSUM_LEN_IN_BYTES;
    if start_of_body > end_of_body || nominal_body_length != body_length {
        dbglog!(
            "BodyLength mismatch: expected {} but is {}.",
            body_length,
            nominal_body_length,
        );
        Err(DecodeError::Invalid)
    } else {
        debug_assert!(body_length < data.len());
        Ok(())
    }
}

pub fn encode_raw<B, F>(
    begin_string: &[u8],
    body_writer: F,
    buffer: &mut B,
    separator: u8,
) -> Result<usize, EncodeError>
where
    B: Buffer,
    F: Fn(&mut B) -> usize,
{
    let start_i = buffer.as_slice().len();
    // First, write `BeginString(8)`.
    buffer.extend_from_slice(b"8=");
    buffer.extend_from_slice(begin_string);
    buffer.extend_from_slice(&[
        separator, b'9', b'=', b'0', b'0', b'0', b'0', b'0', b'0', separator,
    ]);
    let body_length_writable_range = buffer.as_slice().len() - 7..buffer.as_slice().len() - 1;
    let body_length = body_writer(buffer);
    {
        let slice = &mut buffer.as_mut_slice()[body_length_writable_range];
        // The second field is supposed to be `BodyLength(9)`, but obviously
        // the length of the message is unknow until later in the
        // serialization phase. This alone would usually require to
        //
        //  1. Serialize the rest of the message into an external buffer.
        //  2. Calculate the length of the message.
        //  3. Serialize `BodyLength(9)` to `buffer`.
        //  4. Copy the contents of the external buffer into `buffer`.
        //  5. ... go on with the serialization process.
        //
        // Luckily, FIX allows for zero-padded integer values and we can
        // leverage this to reserve some space for the value. We might waste
        // some bytes but the benefits largely outweight the costs.
        //
        // Six digits (~1MB) ought to be enough for every message.
        slice[0] = (body_length / 100000) as u8 + b'0';
        slice[1] = ((body_length / 10000) % 10) as u8 + b'0';
        slice[2] = ((body_length / 1000) % 10) as u8 + b'0';
        slice[3] = ((body_length / 100) % 10) as u8 + b'0';
        slice[4] = ((body_length / 10) % 10) as u8 + b'0';
        slice[5] = (body_length % 10) as u8 + b'0';
    }
    {
        let checksum = checksum_10(&buffer.as_slice()[start_i..]);
        buffer.extend_from_slice(&[
            b'1',
            b'0',
            b'=',
            (checksum / 100) + b'0',
            ((checksum / 10) % 10) + b'0',
            (checksum % 10) + b'0',
            separator,
        ]);
    }
    Ok(buffer.as_slice().len())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn edges_cases_of_checksum_calculation() {
        assert_eq!(checksum_10(&[]), 0);
        assert_eq!(checksum_10(&[1]), 1);
        assert_eq!(checksum_10(&[128, 127]), 255);
        assert_eq!(checksum_10(&[128, 128]), 0);
    }

    #[test]
    fn correct_retrieval_of_checksum_digits() {
        assert_eq!(
            &checksum_digits(b"8=FIX.4.4|9=1337|35=?|...|10=000|"),
            b"000"
        );
        assert_eq!(
            &checksum_digits(b"8=FIX.4.4|9=1337|35=?|...|10=ABC|"),
            b"ABC"
        );
    }
}
