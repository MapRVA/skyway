# Usage

Here is an example of how to use skyway.
For more information, you can run `skyway --help`.

```sh
skyway --from pbf --input input-file.pbf --to json --output output-file.json
```
If you do not specify an input or output file, skyway will default to standard in and standard out, respectively.
This enables you to stream data into and out of skyway, like this:
```sh
cat input-file.pbf | skyway --from pbf --to json > output-file.json
```

## Supported Formats

Here is a table showing the formats skyway supports reading and writing.
The shortname is used to indicate the format when running skyway, e.g. `skyway --from xml ...`

**Key:**<br>
❌ – Not Supported<br>
✅ – Supported<br>
⚡ – Speedy!<br>

| Format | Shortname | Reader | Writer |
| -------|-----------|--------|--------|
| [OPL](https://wiki.openstreetmap.org/wiki/OPL_format) | `opl` | ✅ | ⚡ |
| [OSM JSON](https://wiki.openstreetmap.org/wiki/OSM_JSON) | `json` | ✅ | ⚡ |
| [Overpass JSON](https://wiki.openstreetmap.org/wiki/OSM_JSON#Overpass_API) | † | ✅ | ⚡ |
| [OSM XML](https://wiki.openstreetmap.org/wiki/OSM_XML) | `xml` | ✅ | ✅ |
| [PBF](https://wiki.openstreetmap.org/wiki/PBF_Format) | `pbf` | ⚡ | ❌ |

<sup>†</sup>*Use the shortname `json` to read OSM JSON, it is the same parser. Use `overpass` for writing.*
