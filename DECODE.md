# Decoding configuration

This is an idea about how to deal with a more flexible system enabling more powerful options.

At this moment I feel this is too complex to implement without deviating course, but hopefully
these notes are helpful down the road.

locations:
  - header:
      args:
      ops:
        - lookup:
            	(input: pairs_vec implied by header location type)
            key: x-jwt-payload
		(output: string)
          decode:
            kind: base64url
		(output: string)
          decode:
            kind: json
		(output: json value, maybe string, maybe struct, maybe array)
          lookup:
		(input: json struct, stringy and array-y stuff would error out here)
	    key: aud
	        (output: json string, json struct or json array)
          lookup:
		(input: cannot be a json struct)
            position: 0


when you lookup, you either provide a key/entry (for a struct) or an array position.

Lookup positions are defined for arrays
Lookup positions are defined for strings only if the position is 0.
Lookup positions are defined for Pairs or other structs only if they are ordered.
Everything else has no lookup position.

Lookup keys are defined for structs including Pairs (ie. arrays that behave like a struct such as [(k1, v1), ...]).
Lookup keys are defined for arrays of strings only.
Lookup keys are defined for strings is they match the actual string, ie: "mykey" matches "mykey".
Everything else has no lookup key.

Lookups output either a string, an array or a struct.

Decodes always have strings as input.
Decodes' output format is defined by the kind of decoding.

Lookups must be defined for every potential output format of a decoding.
Decodes must be defined just for strings.

"and" is implied across the list of operations, except if we get an operation type "or" which evaluates each item separately.

The end result of the "pipeline" of operations should either be a string or an array of strings, and there should be
a match based on a key so that:

pipeline_result.lookup(key) returns true if the key is indeed found.

Example of JWT AUTHN filter:

# metadata property
locations:
  - property:
    args:
      path: ["metadata"]
    ops:
      - decode:
          kind: protobuf_struct
        lookup:
          key: filter_metadata
        lookup:
          key: envoy.http.filters.jwt_authn
        lookup:
          position: 0	# name-independent way, because this is (usually??) a pairs with a single entry
        lookup:
          key: aud
        lookup:
          position: 0

# filter_metadata property
locations:
  - property:
    args:
      path: ["metadata", "filter_metadata"]
    ops:
      - decode:
          kind: pairs
        lookup:
          key: envoy.http.filters.jwt_authn
		(output: string - or rather, bytes)
        decode:
          kind: pairs
        lookup:
          position: 0	# this works both for pairs and string, so there needs to be disambiguation! ^ see decode above
			# it could also be, if we know about it, key: verified_jwt
        decode:
          kind: pairs
        lookup:
          key: aud
	# note that at this point we have a string out of aud. but it could also be an array!!
	# we don't really have a way to know so we must try both options
	or:
	  - decode:
	      kind: pairs
		# note: due to the nature of pairs, a string could match a "well-formed" pairs struct
		# this is format-specific, but I can't see a good way out once we are in pairs-land.
          - decode:
              kind: string # this would be a no-op, it would just return the same string, but useful in this context
        lookup:
          position: 0

# verified_jwt property
locations:
  - property:
    args:
      path: ["metadata", "filter_metadata", "envoy.filter.http.jwt_authn", "verified_jwt"]
    ops:
      - decode:
          kind: pairs
        lookup:
          key: aud
	or:
	  - decode:
	      kind: pairs
          - decode:
              kind: string
        lookup:
          position: 0
