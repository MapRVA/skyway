# Filtering

A skyway filter decides whether or not to keep an element, or possibly transforms it.
If you do not provide a filter to skyway, it will convert the original input as faithfully as it can.

## Running skyway with a Filter

To add a filter to skyway, add the `--filter [FILTER FILE]` option.
You may pass multiple filters to evaluate in sequence by passing multiple `--filter` flags.
The file extension does not matter; skyway will detect if the file is in CEL or OSMFilter and parse it accordingly.

## How Filters Work

In skyway, the input reader and output writer are run in different threads, with the former passing element objects to the latter as they become available.
This multithreaded approach makes it easy to add one or more middlemen, who can receive elements from the reader, transform them, and then pass them along down the chain.
skyway provides a "filtering" system to make it easier to add these processing steps in a modular way.
Using any number of CEL or OSMFilter expressions, you can create complex processing pipelines.

This system was inspired by [Pandoc](https://pandoc.org/).
