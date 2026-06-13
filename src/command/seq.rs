//! `seq` - generate a sequence of numbers
//!
//! Prints a sequence of numbers from FIRST to LAST, incrementing by
//! INCREMENT (default 1). With one argument, counts from 1 to LAST.
//! With two arguments, counts from FIRST to LAST. With three arguments,
//! the middle value is the step.
//!
//! # Usage
//!
//! ```text
//! seq LAST
//! seq FIRST LAST
//! seq FIRST INCREMENT LAST
//! ```
//!
//! # Examples
//!
//! ```text
//! seq 5                 # 1 2 3 4 5
//! seq 1 5               # 1 2 3 4 5
//! seq 0 2 10            # 0 2 4 6 8 10
//! seq 5 -1 1            # 5 4 3 2 1
//! ```

/// Maximum number of lines to output, preventing browser hangs from
/// runaway sequences like `seq 1 1000000000`.
const MAX_LINES: usize = 10_000;

/// Execute the `seq` command.
///
/// Parses 1–3 numeric arguments to define a sequence range and step,
/// then generates the sequence as newline-separated output.
///
/// # Arguments
///
/// * `args` - Slice of argument strings: 1 to 3 numeric values.
///
/// # Returns
///
/// `Ok(String)` with the number sequence, or `Err` for invalid arguments.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() || args.len() > 3 {
        return Err("seq: expected 1 to 3 arguments".to_string());
    }

    let (first, step, last) = match args.len() {
        1 => {
            let last: f64 = args[0]
                .parse()
                .map_err(|_| format!("seq: invalid floating point argument: '{}'", args[0]))?;
            (1.0, 1.0, last)
        }
        2 => {
            let first: f64 = args[0]
                .parse()
                .map_err(|_| format!("seq: invalid floating point argument: '{}'", args[0]))?;
            let last: f64 = args[1]
                .parse()
                .map_err(|_| format!("seq: invalid floating point argument: '{}'", args[1]))?;
            (first, 1.0, last)
        }
        3 => {
            let first: f64 = args[0]
                .parse()
                .map_err(|_| format!("seq: invalid floating point argument: '{}'", args[0]))?;
            let step: f64 = args[1]
                .parse()
                .map_err(|_| format!("seq: invalid floating point argument: '{}'", args[1]))?;
            let last: f64 = args[2]
                .parse()
                .map_err(|_| format!("seq: invalid floating point argument: '{}'", args[2]))?;
            (first, step, last)
        }
        _ => unreachable!(),
    };

    if step == 0.0 {
        return Err("seq: step must not be zero".to_string());
    }

    // Determine whether to format as integers (all inputs had no decimal point).
    // Also track the maximum decimal places seen for consistent float formatting.
    let is_integer = args.iter().all(|a| !a.contains('.'));
    let max_decimals = args
        .iter()
        .filter_map(|a| a.find('.').map(|pos| a.len() - pos - 1))
        .max()
        .unwrap_or(0);

    let mut output = String::new();
    let mut current = first;
    let eps = 1e-10;
    let mut count = 0;

    if step > 0.0 {
        while current <= last + eps && count < MAX_LINES {
            if is_integer {
                output.push_str(&format!("{}\n", current as i64));
            } else {
                output.push_str(&format!("{:.1$}\n", current, max_decimals));
            }
            current += step;
            count += 1;
        }
    } else {
        while current >= last - eps && count < MAX_LINES {
            if is_integer {
                output.push_str(&format!("{}\n", current as i64));
            } else {
                output.push_str(&format!("{:.1$}\n", current, max_decimals));
            }
            current += step;
            count += 1;
        }
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `seq`.
pub struct SeqCommand;

impl super::Command for SeqCommand {
    fn name(&self) -> &'static str {
        "seq"
    }

    fn description(&self) -> &'static str {
        "Generate a sequence of numbers"
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(ctx.args).into()
    }

    fn synopsis(&self) -> &'static str {
        "seq LAST | seq FIRST LAST | seq FIRST INCREMENT LAST"
    }

    fn man_description(&self) -> &'static str {
        "Print a sequence of numbers from FIRST to LAST, incrementing by INCREMENT (default 1). \
With one argument, counts from 1 to LAST. With two arguments, counts from FIRST to LAST. \
With three arguments, the middle value is the increment. Negative increments are supported \
for counting down. Output is capped at 10000 lines."
    }

    fn examples(&self) -> &'static [&'static str] {
        &["seq 5", "seq 1 5", "seq 0 2 10", "seq 5 -1 1"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_arg_counts_from_one() {
        let out = execute(&["5"]).unwrap();
        assert_eq!(out, "1\n2\n3\n4\n5\n");
    }

    #[test]
    fn two_args() {
        let out = execute(&["1", "5"]).unwrap();
        assert_eq!(out, "1\n2\n3\n4\n5\n");
    }

    #[test]
    fn three_args_with_step() {
        let out = execute(&["0", "2", "10"]).unwrap();
        assert_eq!(out, "0\n2\n4\n6\n8\n10\n");
    }

    #[test]
    fn negative_step() {
        let out = execute(&["5", "-1", "1"]).unwrap();
        assert_eq!(out, "5\n4\n3\n2\n1\n");
    }

    #[test]
    fn empty_sequence() {
        // first=1, last=0, step=1 => no output
        let out = execute(&["0"]).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn single_number() {
        let out = execute(&["1"]).unwrap();
        assert_eq!(out, "1\n");
    }

    #[test]
    fn too_many_args() {
        assert!(execute(&["1", "2", "3", "4"]).is_err());
    }

    #[test]
    fn no_args() {
        assert!(execute(&[]).is_err());
    }

    #[test]
    fn invalid_number() {
        assert!(execute(&["abc"]).is_err());
    }

    #[test]
    fn zero_step_error() {
        assert!(execute(&["1", "0", "5"]).is_err());
    }

    #[test]
    fn float_sequence() {
        let out = execute(&["0.0", "0.5", "2.0"]).unwrap();
        assert_eq!(out, "0.0\n0.5\n1.0\n1.5\n2.0\n");
    }

    #[test]
    fn negative_range() {
        let out = execute(&["-3", "1"]).unwrap();
        assert_eq!(out, "-3\n-2\n-1\n0\n1\n");
    }
}
