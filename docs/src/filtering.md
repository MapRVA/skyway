# Filtering

A skyway filter decides whether or not to keep an element, or possibly transforms it.
If you do not provide a filter to skyway, it will convert the original input as faithfully as it can.

## Supported Filtering Languages

- [SkyFilter](skyfilter.html)
- [CEL](cel-filters.html)

## Running skyway with a Filter

To add a filter to skyway, add the `--filter [FILTER FILE]` option.
You may pass multiple filters to evaluate in sequence by passing multiple `--filter` flags.

You should use `.skyfilter` or `.cel` file extensions so that skyway can provide helpful parsing error messages. However, this is not necessary and skyway will parse any valid filter file regardless of language.

## How Filters Work

In skyway, the input reader and output writer run in parallel, with the former passing element information to the latter as they become available.
This approach makes it easy to add one or more middlemen, who receive elements from the reader, transform them, and then pass them along down the chain.
skyway provides a "filtering" system to make it easier to add these processing steps in a modular way.
Using any number of CEL or SkyFilter expressions, you can create complex processing pipelines.

This system was inspired by [Pandoc](https://pandoc.org/).
