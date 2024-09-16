# OSMFilter

OSMFilter is a bespoke filtering language designed for skyway, allowing you to transform element data as they pass through skyway.

<div class="warning">
This feature is intended to be the biggest value-add of skyway, but I haven't yet settled on a syntax that feels right.
I would <strong>massively appreciate</strong> your feedback!
How would your ideal filter language work?
</div>

## Specification

An OSMFilter file must start with a header as shown below, with the version matching the version of skyway that you are using, **followed by at least two newlines**.
skyway will warn you if there is a version mismatch.
After the header, you can use any combination of **selectors** and **modifiers** to manipulate the elements.
Every selector must be followed by a **tab-indented** block of one or more modifiers or nested selectors.
Comments start with `#` and extend through the end of a line.
```osmfilter
OSMFilter v0.2.0

TYPE way                                 # selects ways
	HAS "footway"                    # selects elements with a "footway" tag (any value)
		SET "surface" "concrete" # changes the value of the "surface" tag to be "concrete"
		COMMIT                   # immediately commit this element (skip the rest of the filter)
TYPE relation                            # selects relations
	EQUALS "type" "route"            # selects elements with the tag "type" set to "route"
		DROP                     # do not include element in output (skip the rest of the filter)
COMMIT                                   # commit the element
```

## Statements

Below you can find descriptions of each statement supported by OSMFilter.

### Selectors

A selector must be followed by one or more tab-indented statements (either a modifier or selection block).
You can nest selection blocks.

- `TYPE way, node` — Selects elements of specified type(s), in a comma-separated list.
- `HAS "key"` — Selects elements with tag `key`.
- `EQUALS "key" "value"` — Selects elements with tag `key` equalling `value`.

### Modifiers

- `COMMIT` — Commits element as it currently is to be written to output. Short-circuits the rest of the filter for that element.
- `DROP` — Drops element, i.e. excluding it entirely from the output. Short-circuits the rest of the filter for that element.
- `SET "key" "value"` — Sets tag `key` to `value`.
- `KEEP "key_one", "key_two"` — Only keep tags with specified key(s), removing any others from the element.
- `RENAME "oldkey" "newkey"` — Renames tag `oldkey` to `newkey`, keeping the value of the tag the same.
- `DELETE "key_one", "key_two"` — Removes tag(s) with specified key(s) from the element.
