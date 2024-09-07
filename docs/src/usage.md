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

| Format        | Shortname | Reader | Writer |
| --------------|-----------|--------|--------|
| GeoJSON       |           | ➖     | ❌     |
| o5m           |           | ❌     | ❌     |
| OPL           | `opl`     | ✅     | ✅     |
| OSM JSON      | `json`    | ✅     | ✅     |
| Overpass JSON | †         | ✅     | ❌     |
| OSM XML       | `xml`     | ✅     | ✅     |
| PBF           | `pbf`     | ✅     | ❌     |

<sup>†</sup>*Use the shortname `json` as if to read OSM JSON, it is the same parser.*
