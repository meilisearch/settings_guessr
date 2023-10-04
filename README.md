# Settings Guessr 🤔

## Context

- By default, every field is considered **searchable**. Ex: URLs, prices, dates.
- By default, no field is considered **filterable**. Ex: users will not be able to sort by price.
- Users raised issues related to indexing performance, and best practices with regards to how the index settings may impact the experience
- Limited knowledge of the different index settings lead to user confusion, anger, and sometimes violence

## Solution

- Rules to detect common field values, such as URLs, UUIDs
- Field name lookups to understand the usage of each field, such as `id`, datetimes, and so on
- Entropy used to measure the amount of randomness of certain values over the whole dataset
  - if we observe a small amount of different values in the entire dataset, the field is probably a "category" (color, genres, ...), and thus should be filterable
  - if we observe a large amount of different values, the field is likely to represent unique content, thus should be searchable

### Implementation

- Base library in Rust
  - can be shipped on the web (fast client-side validation with WASM)
  - can be embedded inside of Meilisearch in the future
  - directly embedded in the Cloud 🦭

## Demo

[SettingsGuessr tool](https://meilisearch.github.io/settings_guessr/)

## Bonus

https://meilisearch-cloud-staging-pvc3tikmm-meili.vercel.app

## Impact

### Easier new feature onboarding

Users do not need to be aware of new settings and features enable via settings: we will offer them the best experience by default.

### Better search and indexing performance

| dataset                       | count | default | generated | Faster by |
| ----------------------------- | ----- | ------- | --------- | --------- |
| All cities                    | 1M    | 40.02s  | 37.38s    | 6.6%      |
| Hacker News                   | 1M    | 184.51s | 170.99s   | 7.33%     |
| Movies                        | 32k   | 4.12s   | 3.72s     | 9.71%     |
| Magic The Gathering card game | 74k   | 35.71s  | 19.91s    | 44.25%    |
| Twitch messages               | 5M    | 248.58s | 111.80s   | 55.03%    |
| Music Brainz                  | 100k  | 83.80s  | 35.86s    | 57.21%    |

## What's next?

- Language detection, in order to download synonyms and stop words accordingly
- Continuous analysis of documents to keep settings as up-to-date as possible
- Provide an explanation for all suggested settings to the user

## Special thanks

MrBeast 🫶
