fn exponential_notation(
    leading_digit: &str,
    fractional_digits: &str,
    exponent_sign: char,
    exponent_digits: &str,
) -> String {
    let mantissa = if fractional_digits.is_empty() {
        leading_digit.to_owned()
    } else {
        format!("{leading_digit}.{fractional_digits}")
    };

    format!("{mantissa}e{exponent_sign}{exponent_digits:0>2}")
}

fn fractional_notation(
    leading_digit: &str,
    fractional_digits: &str,
    negative_exponent_magnitude: usize,
) -> String {
    format!("0.{leading_digit:0>negative_exponent_magnitude$}{fractional_digits}")
}

fn integral_notation(leading_digit: &str, fractional_digits: &str, exponent: usize) -> String {
    match fractional_digits.split_at_checked(exponent) {
        Some((integer_digits, decimal_digits)) if !decimal_digits.is_empty() => {
            format!("{leading_digit}{integer_digits}.{decimal_digits}")
        }
        _ => format!("{leading_digit}{fractional_digits:0<exponent$}.0"),
    }
}

#[derive(Clone, Copy)]
enum ScientificNotationPart {
    Exponent,
    FractionalDigits,
    LeadingDigit,
}

struct ScientificDigits {
    exponent_digits: String,
    exponent_is_negative: bool,
    fractional_digits: String,
    leading_digit: String,
}

impl ScientificDigits {
    fn parse(scientific_notation: &str) -> Self {
        let mut part = ScientificNotationPart::LeadingDigit;
        let mut exponent_digits = String::new();
        let mut exponent_is_negative = false;
        let mut fractional_digits = String::new();
        let mut leading_digit = String::new();

        for byte in scientific_notation.bytes() {
            match (part, byte) {
                (
                    ScientificNotationPart::LeadingDigit | ScientificNotationPart::FractionalDigits,
                    b'e',
                ) => part = ScientificNotationPart::Exponent,
                (ScientificNotationPart::LeadingDigit, b'.') => {
                    part = ScientificNotationPart::FractionalDigits;
                }
                (ScientificNotationPart::LeadingDigit, digit) => {
                    leading_digit.push(char::from(digit));
                }
                (ScientificNotationPart::FractionalDigits, digit) => {
                    fractional_digits.push(char::from(digit));
                }
                (ScientificNotationPart::Exponent, b'-') => exponent_is_negative = true,
                (ScientificNotationPart::Exponent, digit) => {
                    exponent_digits.push(char::from(digit));
                }
            }
        }

        Self {
            exponent_digits,
            exponent_is_negative,
            fractional_digits,
            leading_digit,
        }
    }

    fn shortest_with_ties_to_even(magnitude: f64) -> Self {
        let shortest = format!("{magnitude:e}");
        let shortest_digits = Self::parse(&shortest);
        let ties_to_even = format!(
            "{magnitude:.precision$e}",
            precision = shortest_digits.fractional_digits.len()
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
    let ScientificDigits {
        exponent_digits,
        exponent_is_negative,
        fractional_digits,
        leading_digit,
    } = ScientificDigits::shortest_with_ties_to_even(value.abs());
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let unsigned = match (exponent_is_negative, exponent_digits.parse::<usize>()) {
        (true, Ok(exponent_magnitude @ 1..=4)) => {
            fractional_notation(&leading_digit, &fractional_digits, exponent_magnitude)
        }
        (false, Ok(exponent @ 0..=15)) => {
            integral_notation(&leading_digit, &fractional_digits, exponent)
        }
        (true, _) => {
            exponential_notation(&leading_digit, &fractional_digits, '-', &exponent_digits)
        }
        (false, _) => {
            exponential_notation(&leading_digit, &fractional_digits, '+', &exponent_digits)
        }
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
