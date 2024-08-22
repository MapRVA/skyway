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
For now, speed is not currently one of the goals for this package.
With that said, I welcome any contributions to this project that might speed it up!

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
    - [ ] writer _(works, but does not support document metdata yet!)_
    - [ ] explicitly support both overpass and official OSM JSON
  - OSM XML
    - [ ] reader
    - [ ] writer
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
