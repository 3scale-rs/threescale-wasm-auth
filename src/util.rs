pub mod pairs;

use protobuf::Message;

pub fn serde_json_error_lines<'i, 'e: 'i>(
    e: &'e serde_json::Error,
    input: &'i str,
    before_ctx: usize,
    after_ctx: usize,
) -> impl Iterator<Item = String> + 'i {
    let line = e.line();
    // this is not a parsing error (ie. programmatic)
    assert_ne!(line, 0);
    let column = e.column();
    let line_skip = line.checked_sub(before_ctx.saturating_add(1)).unwrap_or(0);
    // before_len also takes the error line
    let before_len = line - line_skip;
    let numchars = |mut num: usize| {
        let mut chars: usize = 1;
        while num > 9 {
            num /= 10;
            chars += 1;
        }
        chars
    };
    let last_line = (0..=after_ctx)
        .rev()
        .find_map(|after| line.checked_add(after))
        .unwrap_or(line);
    let after_len = last_line - line;
    let lineno_width = numchars(last_line);
    let format_line = move |(current_line, line)| {
        format!(
            "{:>width$}: {}",
            current_line + line_skip + 1,
            line,
            width = lineno_width
        )
    };
    let before_it = input.lines().skip(line_skip);
    let after_it = before_it
        .clone()
        .enumerate()
        .skip(before_len)
        .take(after_len)
        .map(format_line);

    before_it
        .enumerate()
        .take(before_len)
        .map(format_line)
        .chain(core::iter::once(format!(
            "{: >width$}  {: >columns$} error ({:?}) {}",
            "",
            "^",
            e.classify(),
            e,
            width = lineno_width,
            columns = column
        )))
        .chain(after_it)
}

pub fn serde_json_error_to_string<'i, 'e: 'i>(e: &'e serde_json::Error, input: &'i str) -> String {
    serde_json_error_lines(e, input, 2, 2)
        .collect::<Vec<_>>()
        .join("\n")
}

fn jwt_parts(jwt: &str) -> (&str, &str, &str) {
    let mut it = jwt.split('.');
    let header = it.next().unwrap();
    let payload = it.next().unwrap();
    let signature = it.next().unwrap();

    (header, payload, signature)
}

fn jwt_parse() {
    let jwt = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJoLTJ5ZV9lVjZHZllUMTg2N0xuM01ETW96SUU0aXlJTHVkcUhuMFJDNlFRIn0.eyJleHAiOjE2MTQ2MjA5MjcsImlhdCI6MTYxNDYyMDg2NywiYXV0aF90aW1lIjoxNjE0NjIwODY2LCJqdGkiOiIzMjJkYWNlNS03NWVlLTQzMGItOWNhZC0yYWEwNGMyYjM1MjciLCJpc3MiOiJodHRwczovL2tleWNsb2FrOjg0NDMvYXV0aC9yZWFsbXMvbWFzdGVyIiwiYXVkIjoidGVzdCIsInN1YiI6ImE3MTUyYWY0LTZjYjQtNDhkYi1iMjhmLWNiNGU3YWYxMWI1OSIsInR5cCI6IklEIiwiYXpwIjoidGVzdCIsInNlc3Npb25fc3RhdGUiOiI1MWZmMDQyMy04M2QzLTQ0MjQtYjI4Ni0xNWZiOTcyMmY4NDUiLCJhdF9oYXNoIjoibkZzWHJEVG9QMmh5Qi1uU25wemRodyIsImFjciI6IjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6ImFkbWluIn0.bgT1Z_aVGl3_xzUDxLuwmRpI8se_fIdpQVCDO3uEEbcQndFJ-clDdb4d5sfqaEQrCC0ezOVCFNmRr0fn4fgKb_ewsK8ZFBOa-PKSgViqymAxlhPWRWHFllNJHk6tCw83Q9Y5EI99_qp-dy2Wal_vvzJ2cHwz9tjuD2169Y69NHXoUDt3ABFHnczC4hiMIrHPgqFmQbmcIyc7n36D9abCBdb9dVPBgTMVKM-NLYK-3f_uEJ1M9ZGxEyTDDDC4WGLkskTaXwPh9C0Cbz_1ZZEoFFldOQHC_uV5LsKMZAjEWm2PjAoB-OomKImXbWz16Mw5gXofwRaxET11XRLCyGNviw";
    let jwt_parts = jwt.split('.').collect::<Vec<_>>();
    assert_eq!(jwt_parts.len(), 3);
    let jwt_first = base64::decode_config(jwt_parts[0], base64::URL_SAFE);
    assert!(jwt_first.is_ok());
    let jwt_first = jwt_first.unwrap();
    // generate message with something like prost::json::StringToMessage(&jwt_first)
    let jwt_first_s = unsafe { String::from_utf8_unchecked(jwt_first) };
    let jwt_first_pb =
        protobuf::json::parse_from_str::<protobuf::well_known_types::Struct>(jwt_first_s.as_str());
    assert!(jwt_first_pb.is_ok());
    let jwt_first_pb = jwt_first_pb.unwrap();
    let jwt_first_fields = &jwt_first_pb.fields;
    let alg = jwt_first_fields.get("alg");
    assert!(alg.is_some());
    let alg = alg.unwrap();
    match &alg.kind {
        Some(protobuf::well_known_types::value::Kind::string_value(s)) => {
            eprintln!("matching the value of alg is {}", s)
        }
        Some(v) => {
            eprintln!("value which should have been string is not! it is {:#?}", v);
        }
        None => (),
    }
    assert!(alg.has_string_value());
    let alg_s = alg.get_string_value();
    eprintln!("alg is {}", alg_s);
    // kid if present must be a string
    match jwt_first_fields.get("kid") {
        Some(kid) => {
            assert!(kid.has_string_value());
            eprintln!("kid is {}", kid.get_string_value());
        }
        None => (),
    }
    let jwt_payload = base64::decode_config(jwt_parts[1], base64::URL_SAFE);
    assert!(jwt_payload.is_ok());
    let jwt_payload = jwt_payload.unwrap();
    let jwt_payload_s = unsafe { String::from_utf8_unchecked(jwt_payload) };
    let jwt_payload_pb = protobuf::json::parse_from_str::<protobuf::well_known_types::Struct>(
        jwt_payload_s.as_str(),
    );
    assert!(jwt_payload_pb.is_ok());
    let jwt_payload_pb = jwt_payload_pb.unwrap();
    let bytes_out = jwt_payload_pb.write_to_bytes();
    assert!(
        bytes_out.is_ok(),
        "cannot create bytes vector out of payload pb"
    );
    let bytes_out = bytes_out.unwrap();
    let hex = bytes_out
        .iter()
        .map(|c| format!("{:#02x}", *c))
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!("Payload PB bytes (len {}): [{}]", bytes_out.len(), hex);
    let jwt_payload_fields = &jwt_payload_pb.fields;
    ["iss", "sub", "jti"]
        .iter()
        .for_each(|&f| match jwt_payload_fields.get(f) {
            Some(v) => {
                assert!(v.has_string_value());
                eprintln!("{} is {}", f, v.get_string_value());
            }
            None => eprintln!("{} is not present", f),
        });
    ["iat", "nbf", "exp"]
        .iter()
        .for_each(|&f| match jwt_payload_fields.get(f) {
            Some(v) => {
                assert!(v.has_number_value());
                eprintln!("{} is {}", f, v.get_number_value());
            }
            None => eprintln!("{} is not present", f),
        });
    // aud can be a string or a list of strings, or empty _iff_ azp is present
    let aud = match jwt_payload_fields.get("aud") {
        Some(v) => {
            if v.has_string_value() {
                vec![v.get_string_value()]
            } else {
                assert!(v.has_list_value());
                let v = v.get_list_value();
                v.values
                    .iter()
                    .map(|v| {
                        // all items in the list of values should be strings
                        assert!(v.has_string_value());
                        v.get_string_value()
                    })
                    .collect::<Vec<_>>()
            }
        }
        None => {
            vec![]
        }
    };
    eprintln!("aud is {:?}", aud);
    let azp = match jwt_payload_fields.get("azp") {
        Some(v) => {
            assert!(v.has_string_value());
            let v = v.get_string_value();
            eprintln!("azp is {}", v);
            v
        }
        None => {
            assert!(!aud.is_empty(), "both aud and azp cannot be empty");
            ""
        }
    };
    let app_id = if azp.is_empty() { aud[0] } else { azp };
    eprintln!("app_id is {}", app_id);
    assert!(!app_id.is_empty());
    let jwt_signature = base64::decode_config(jwt_parts[2], base64::URL_SAFE);
    assert!(jwt_signature.is_ok());
    let jwt_signature = jwt_signature.unwrap();
    let jwt_signature_s = String::from_utf8_lossy(jwt_signature.as_slice());
    // JWT signature is not JSON and should not be loaded into a protobuf struct
    eprintln!("JWT signature: {}", jwt_signature_s);
}
