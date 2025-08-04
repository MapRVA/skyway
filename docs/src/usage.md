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

Here is a table showing the formats skyway supports reading or writing.
The shortname is used to indicate the format when running skyway, e.g. `skyway --from xml ...`.
When paired together, "speedy" readers and writers will generally run faster by passing data between threads.

**Key:**<br>
❌ – Not Supported<br>
✅ – Supported<br>
⚡ – Speedy!<br>

| Format | Shortname | Reader | Writer |
| -------|-----------|--------|--------|
| [o5m](https://wiki.openstreetmap.org/wiki/O5m) | `o5m` | ❌ | ✅ |
| [OPL](https://wiki.openstreetmap.org/wiki/OPL_format) | `opl` | ⚡ | ⚡ |
| [OSM JSON](https://wiki.openstreetmap.org/wiki/OSM_JSON) | `json` | ✅ | ⚡ |
| [OSM XML](https://wiki.openstreetmap.org/wiki/OSM_XML) | `xml` | ✅ | ⚡ |
| [PBF](https://wiki.openstreetmap.org/wiki/PBF_Format) | `pbf` | ⚡ | ❌ |
