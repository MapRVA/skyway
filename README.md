# skyway

skyway is a command-line OpenStreetMap file converter.

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
    - [ ] writer
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

## Alternatives

Before adopting skyway for your project, please also consider [Osmium Tool](https://osmcode.org/osmium-tool/), a mature and well-trusted application that accomplished many of the same things.

For a complete list of alternatives, please see the [OSM file formats page](https://wiki.openstreetmap.org/wiki/OSM_file_formats) on the OpenStreetMap wiki.

## License

skyway is released under the GPLv3+ license.
Please see LICENSE.md for more information.
