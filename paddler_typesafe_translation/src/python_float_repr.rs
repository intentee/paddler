fn exponential_notation(digits: &str, exponent: i64) -> String {
    let exponent_sign = if exponent < 0 { '-' } else { '+' };
    let mantissa = if digits.len() > 1 {
        format!("{}.{}", &digits[..1], &digits[1..])
    } else {
        digits.to_owned()
    };

    format!("{mantissa}e{exponent_sign}{:02}", exponent.abs())
}

fn positional_notation(digits: &str, decimal_point_position: i64) -> String {
    if decimal_point_position <= 0 {
        return format!(
            "0.{}{digits}",
            "0".repeat((-decimal_point_position) as usize)
        );
    }

    let integer_digit_count = decimal_point_position as usize;

    if integer_digit_count >= digits.len() {
        format!(
            "{digits}{}.0",
            "0".repeat(integer_digit_count - digits.len())
        )
    } else {
        format!(
            "{}.{}",
            &digits[..integer_digit_count],
            &digits[integer_digit_count..]
        )
    }
}

struct ScientificDigits {
    digits: String,
    exponent: i64,
}

impl ScientificDigits {
    fn parse(scientific_notation: &str) -> Self {
        let mut digits = String::new();
        let mut exponent_magnitude = 0_i64;
        let mut exponent_is_negative = false;
        let mut reading_exponent = false;

        for character in scientific_notation.chars() {
            match (reading_exponent, character) {
                (false, 'e') => reading_exponent = true,
                (false, '.') => {}
                (false, digit) => digits.push(digit),
                (true, '-') => exponent_is_negative = true,
                (true, digit) => {
                    exponent_magnitude =
                        exponent_magnitude * 10 + i64::from(u32::from(digit) - u32::from('0'));
                }
            }
        }

        Self {
            digits,
            exponent: if exponent_is_negative {
                -exponent_magnitude
            } else {
                exponent_magnitude
            },
        }
    }

    fn shortest_with_ties_to_even(magnitude: f64) -> Self {
        let shortest = format!("{magnitude:e}");
        let shortest_digits = Self::parse(&shortest);
        let ties_to_even = format!(
            "{magnitude:.precision$e}",
            precision = shortest_digits.digits.len() - 1
        );

        if ties_to_even.parse::<f64>() == Ok(magnitude) {
            Self::parse(&ties_to_even)
        } else {
            shortest_digits
        }
    }
}

#[must_use]
pub fn python_float_repr(value: f64) -> String {
    let ScientificDigits { digits, exponent } =
        ScientificDigits::shortest_with_ties_to_even(value.abs());
    let decimal_point_position = exponent + 1;
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let unsigned = if decimal_point_position <= -4 || decimal_point_position > 16 {
        exponential_notation(&digits, exponent)
    } else {
        positional_notation(&digits, decimal_point_position)
    };

    format!("{sign}{unsigned}")
}

#[cfg(test)]
mod tests {
    use super::python_float_repr;

    #[test]
    fn formats_floats_the_way_python_repr_does() {
        for (value, python_repr) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (100.0, "100.0"),
            (19.99, "19.99"),
            (0.1, "0.1"),
            (0.0001, "0.0001"),
            (1e-05, "1e-05"),
            (1e-07, "1e-07"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1.5e300, "1.5e+300"),
            (-2.5, "-2.5"),
            (123_456_789.123, "123456789.123"),
            (123.456_789_012_345_67, "123.45678901234567"),
            (1_000_000_000_000.656_2, "1000000000000.6562"),
            (7.120_236_347_223_045e-307, "7.120236347223045e-307"),
        ] {
            assert_eq!(python_float_repr(value), python_repr, "{value}");
        }
    }
}
