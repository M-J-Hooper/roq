# `rq`

A rust implementation of the popular command-line JSON processor, [`jq`](https://github.com/stedolan/jq).

## Installation

`cargo install --git https://github.com/M-J-Hooper/rq`

## Example

`curl 'https://api.github.com/repos/M-J-Hooper/rq/commits' | rq '.[].sha'`

## Implementation

1. Parse JSON input using [`serde_json`](https://github.com/serde-rs/json).
2. Parse `jq` query syntax into a `Query` using a recursive parser built with [`nom`](https://github.com/Geal/nom).
3. Execute query against the JSON value which recursively propagates through the nested queries to produce JSON values as a result.

This was mostly done as a learning exercise and as such it does not support some of the more obscure (and less useful) features of the original. However, it is likely feature-complete enough for day-to-day use.
