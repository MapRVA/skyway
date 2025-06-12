# SkyFilter

SkyFilter is a bespoke filtering language designed for skyway, allowing you to transform element data as they pass through skyway.

<div class="warning">
This feature is intended to be the biggest value-add of skyway, but I haven't yet settled on a syntax that feels right.
I would <strong>massively appreciate</strong> your feedback!
How would your ideal filter language work?
</div>

## Specification

A SkyFilter file must start with a header as shown below, with the version matching the version of skyway that you are using, **followed by a blank line**.
After the header, you can use any combination of **selectors** and **modifiers** to manipulate the elements.
Every selector must be followed by a **tab-indented** block of one or more modifiers or nested selectors.
Comments start with `#` and extend through the end of a line.

It is highly recommended to use the extension `.skyfilter` when saving SkyFilter files so that skyway knows what kind of filter it's intended to be.
When you do this, skyway will provide more detailed parsing errors.
Additionally, skyway will warn you if its version does not match that in the header of your SkyFilter file.

```skyfilter
SkyFilter v0.7.1

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

Below you can find descriptions of each statement supported by SkyFilter.

### Selectors

A selector must be followed by one or more tab-indented statements (either a modifier or selection block).
You can nest selection blocks to create more complex filters.

| Selector                                    | Description |
|---------------------------------------------|-------------|
| <code>TYPE&nbsp;node,&nbsp;relation</code>  | Selects elements of specified type(s), in a comma-separated list. These can be `node`, `way`, or `relation` (no surrounding quotes). |
| <code>HAS&nbsp;"key"</code>                 | Selects elements with tag `key`. This can be a regular expression (regex, see below). |
| <code>EQUALS&nbsp;"key"&nbsp;"value"</code> | Selects elements with tag `key` equalling `value`. The `value` may be a regular expression. |

### Modifiers

| Modifier                                     | Description |
|----------------------------------------------|-------------|
| <code>COMMIT</code>                          | Immediately commits element in its current state to output. |
| <code>DROP</code>                            | Immediately drops element, excluding it entirely from the output. |
| <code>SET&nbsp;"key"&nbsp;"value"</code>     | Sets key to value, creating a new tag if necessary. |
| <code>DELETE&nbsp;"key1",&nbsp;"key2"</code> | Removes tag(s) with specified key(s) from the element. These may be regular expressions. |
| <code>KEEP&nbsp;"key1",&nbsp;"key2"</code>   | Only keeps tags with specified key(s), deleting any others from the element. These may be regular expressions. |
| <code>RENAME&nbsp;"old"&nbsp;"new"</code>    | If a tag exists with the first given key, the key is replaced with the second given key. Any existing tag with that key is overwritten. |

## Using Regular Expressions (Regex)

Many SkyFilter statements accept regular expressions (see above).
While regular strings are wrapped in double quotes (e.g. `"footway"`) regex strings are prefixed with `r` before the first quote, like this: `r"^tiger:"`.

SkyFilter uses the [regex](https://docs.rs/regex/latest/regex/) crate to parse regex expressions. See [here](https://docs.rs/regex/latest/regex/#syntax) for a detailed description of its syntax.
