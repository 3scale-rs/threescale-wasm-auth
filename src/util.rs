pub fn serde_json_error_lines<'i, 'e: 'i>(
    e: &'e serde_json::Error,
    input: &'i str,
) -> impl Iterator<Item = String> + 'i {
    let lineno = e.line().checked_sub(1).unwrap_or(0);
    let line_skip = lineno.checked_sub(1).unwrap_or(0);
    let line_max = e.line().checked_add(1).unwrap_or(e.line());
    let column = e.column();
    let numchars = |mut num: usize| {
        let mut chars: usize = 1;
        while num > 9 {
            num /= 10;
            chars += 1;
        }
        chars
    };
    let lineno_width = numchars(line_max);
    input
        .lines()
        .enumerate()
        .skip(line_skip)
        .take(3)
        .map(move |(current_line, line)| {
            if current_line == lineno {
                format!(
                    "{:>width$}: {}\n{: >width$}  {: >columns$} error ({:?}) {}",
                    current_line,
                    line,
                    "",
                    "^",
                    e.classify(),
                    e,
                    width = lineno_width,
                    columns = column
                )
            } else {
                format!("{:>width$}: {}", current_line, line, width = lineno_width)
            }
        })
}

pub fn serde_json_error_to_string<'i, 'e: 'i>(e: &'e serde_json::Error, input: &'i str) -> String {
    serde_json_error_lines(e, input)
        .collect::<Vec<_>>()
        .join("\n")
}
