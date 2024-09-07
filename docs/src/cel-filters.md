# CEL Filters

skyway supports the [Common Expression Language](https://cel.dev/) (CEL) for filtering elements.
Each time the filter is evaluated for a given element, that expression's context (local variables) is updated to match the element's metadata.
For now, **CEL filters may only return a boolean value**, indicating whether or not the element should be kept.
Please [file an issue](https://github.com/MapRVA/skyway/issues) if you'd like to see more complex CEL return types supported.

## CEL Context

The following table describes each variable available to your expression:

| Variable Name  | CEL Type                                      |
| -------------- | --------------------------------------------- |
| `tags`         | `map` with `string` keys and `string` values  |
| `changeset`    | `int`                                         |
| `user`         | `string`                                      |
| `uid`          | `int`                                         |
| `id`           | `int`                                         |
| `timestamp`    | `string`                                      |
| `visible`      | `bool`                                        |
| `type`         | `string` ("node", "way", or "relation")       |
