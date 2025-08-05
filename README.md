# skyway

[![tests badge](https://github.com/MapRVA/skyway/actions/workflows/test.yml/badge.svg)](https://github.com/MapRVA/skyway/actions/workflows/test.yml)
[![crates.io](https://img.shields.io/crates/v/skyway.svg)](https://crates.io/crates/skyway)

skyway is a command-line OpenStreetMap data conversion and filtering utility.

> [!WARNING]
> skyway is currently in <a href="https://semver.org/">major version (0.y.z)</a>, i.e. it is undergoing initial development.
While I hope you find skyway helpful, it might not work as expected.
Your feedback and contributions are welcome. Thanks!

## Documentation

Please see the [skyway documentation](https://maprva.github.io/skyway/) to learn more!

Join `#skyway` in the [OpenStreetMap Slack](https://openstreetmap.us/get-involved/slack/) to participate in general discussion about skyway.

## Supported Formats

**Key:**<br>
❌ – Not Supported (Yet)<br>
➖ – Not Planned<br>
✅ – Supported<br>

| Format | Reader | Writer |
| -------|--------|--------|
| [GeoJSON](https://wiki.openstreetmap.org/wiki/GeoJSON) | ➖ | ❌ |
| [GeoPackage](https://wiki.openstreetmap.org/wiki/GeoPackage) | ➖ | ❌ |
| GeoParquet | ❌ | ❌ |
| [GOL](https://wiki.openstreetmap.org/wiki/Geographic_Object_Library) | ❌ | ❌ |
| [o5m](https://wiki.openstreetmap.org/wiki/O5m) | ❌ | ✅ |
| [OPL](https://wiki.openstreetmap.org/wiki/OPL_format) | ✅ | ✅ |
| [OSM Express](https://wiki.openstreetmap.org/wiki/OSM_Express) | ❌ | ❌ |
| [OSM JSON](https://wiki.openstreetmap.org/wiki/OSM_JSON) | ✅     | ✅     |
| [Overpass JSON](https://wiki.openstreetmap.org/wiki/OSM_JSON#Overpass_API) | ❌ | ❌ |
| [OSM XML](https://wiki.openstreetmap.org/wiki/OSM_XML) | ✅ | ✅ |
| [PBF](https://wiki.openstreetmap.org/wiki/PBF_Format) | ✅ | ❌ |

## Contributing

Before contributing, please review our [code of conduct](CODE_OF_CONDUCT.md).

Thank you for your interest in contributing to skyway!
[Issues](https://github.com/MapRVA/skyway/issues), [pull requests](https://github.com/MapRVA/skyway/pulls), and [emails](mailto:email@jacobhall.net) are all welcome.
If you would like to make drastic changes to skyway, I recommend reaching out first so that we can coordinate our efforts.

## License

skyway is released under GPLv3 or any later version.
Please see [LICENSE.md](LICENSE.md) for more information.

Test data in the `tests/` directory is derived from [OpenStreetMap](https://www.openstreetmap.org), and is therefore subject to the [Open Database License (ODbL)](https://opendatacommons.org/licenses/odbl/).
Please click [here](https://www.openstreetmap.org/copyright) for more information.
