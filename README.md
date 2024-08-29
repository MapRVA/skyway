# skyway

skyway is a command-line OpenStreetMap file converter.

> [!WARNING]
> skyway is currently in [major version (0.y.z)](https://semver.org/), i.e. it is undergoing initial development.
> While I hope skyway is helpful to you, please expect its API and featureset to change at any time.
> Your feedback and contributions are welcome, see below. Thanks!

## Motivation

This project has a few goals:
- Experiment with a (hopefully) user-friendly tag filtering system
- Support a wide range of OpenStreetMap file formats
- Develop a deeper understanding of the OpenStreetMap data model
- Learn a new programming language

Perhaps one day this tool will be fast enough to compete with other OpenStreetMap conversion software.
For now, speed is not among my primary goals.
With that said, I welcome any performance-minded contributions!

## Usage

Here is an example of how to use skyway.
For more information, you can run `skyway --help`.

```sh
skyway --from pbf --input input-file.pbf --to json --output output-file.json
```
If you do not specify an input or output file, skyway will default to standard in and standard out, respectively.
This enables you to stream data into and out of skyway, like this;
```sh
cat input-file.pbf | skyway --from pbf --to json > output-file.json
```

### Filtering Elements

OSMFilter is an experimental filter format, providing a language for transforming element data as they pass through skyway.
If you do not provide a filter to skyway, it will convert the original input as faithfully as it can.

> [!IMPORTANT]
> This feature is intended to be the biggest value-add of skyway, but I haven't yet settled on a syntax that feels right.
> I would **massively appreciate** your feedback!
> How would your ideal filter language work?

#### Running skyway With a Filter

To add a filter to skyway, add the `--filter [FILTER FILE]` option.

#### OSMFilter Syntax

An OSMFilter file must start with a header as shown below, with the version matching the version of skyway that you are using, **followed by at least two newlines**.
skyway will warn you if there is a version mismatch.
After the header, you can use any combination of **selectors** and **modifiers** to manipulate the elements.
Every selector must be followed by a **tab-indented** block of one or more modifiers or nested selectors.
Comments start with `#` and extend through the end of a line.
```
OSMFilter v0.0.1

TYPE way                                 # selects ways
	HAS "footway"                    # selects elements with a "footway" tag (any value)
		SET "surface" "concrete" # changes the value of the "surface" tag to be "concrete"
		COMMIT                   # immediately commit this element (skip the rest of the filter)
TYPE relation                            # selects relations
	EQUALS "type" "route"            # selects elements with the tag "type" set to "route"
		DROP                     # do not include element in output (skip the rest of the filter)
COMMIT                                   # commit the element
```

#### Selectors

A selector must be followed by one or more tab-indented statements (either a modifier or selection block).
You can nest selection blocks.
- `TYPE way, node` — Selects elements of specified type(s), in a comma-separated list.
- `HAS "key"` — Selects elements with tag `key`.
- `EQUALS "key" "value"` — Selects elements with tag `key` equalling `value`.

#### Modifiers

- `COMMIT` — Commits element as it currently is to be written to output. Short-circuits the rest of the filter for that element.
- `DROP` — Drops element, i.e. excluding it entirely from the output. Short-circuits the rest of the filter for that element.
- `SET "key" "value"` — Sets tag `key` to `value`.
- `RENAME "oldkey" "newkey"` — Renames tag `oldkey` to `newkey`, keeping the value of the tag the same.
- `DELETE "key"` — Removes tag `key` from the element.

## Roadmap

- File types
  - GeoJSON
    - [ ] writer
  - o5m
    - [ ] reader
    - [ ] writer
  - OPL
    - [ ] reader
    - [ ] writer
  - OSM JSON
    - [X] reader
    - [X] writer
    - [ ] explicitly support both overpass and official OSM JSON
  - OSM XML
    - [X] reader
    - [X] writer
  - PBF
    - [X] reader
    - [ ] writer
- Filtering
  - [X] Add basic filtering support
  - [ ] Iterate after getting feedback on filter syntax / featureset
  - [ ] Support multiple filters in sequence
- Change files
  - [ ] Investigate supporting change file input/output
- Testing
  - [ ] Begin writing test suite
- Build
  - [ ] Document build process for Linux
  - [ ] Release binary for Linux
  - [ ] Investigate supporting other platforms

## Contributing

Before contributing, please review our [code of conduct](CODE_OF_CONDUCT.md).

Thank you for your interest in contributing to skyway!
Issues, pull requests, and email communication are all welcome.
If you would like to make drastic changes to skyway, I recommend reaching out first so that we can coordinate our efforts.

## Alternatives

Before adopting skyway for your project, please also consider [Osmium Tool](https://osmcode.org/osmium-tool/), a mature and well-trusted application that accomplished many of the same things.

For a complete list of alternatives, please see the [OSM file formats page](https://wiki.openstreetmap.org/wiki/OSM_file_formats) on the OpenStreetMap wiki.

## License

skyway is released under the GPLv3+ license.
Please see [LICENSE.md](LICENSE.md) for more information.

Example data in this repository is from [OpenStreetMap](https://www.openstreetmap.org), and is therefore subject to the [Open Database License (ODbL)](https://opendatacommons.org/licenses/odbl/).
Please click [here](https://www.openstreetmap.org/copyright) for more information.
