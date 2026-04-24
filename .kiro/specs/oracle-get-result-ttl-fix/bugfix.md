# Bugfix Requirements Document

## Introduction

`get_result` reads a `ResultEntry` from persistent storage but never calls `extend_ttl` on the entry. Because persistent storage entries expire after `MATCH_TTL_LEDGERS` ledgers without a refresh, repeated reads without a new `submit_result` call will silently allow the entry to be evicted. This means the escrow contract's cross-contract call to `get_result` can fail after the TTL lapses even though the result was legitimately submitted, breaking the payout flow.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN `get_result` is called for a match whose result entry exists in persistent storage THEN the system returns the entry without extending its TTL
1.2 WHEN `get_result` is called repeatedly over time without a new `submit_result` call THEN the system allows the persistent storage entry to expire, making subsequent reads return `Error::ResultNotFound`

### Expected Behavior (Correct)

2.1 WHEN `get_result` is called for a match whose result entry exists in persistent storage THEN the system SHALL extend the TTL of that entry by `MATCH_TTL_LEDGERS` before returning it
2.2 WHEN `get_result` is called repeatedly over time without a new `submit_result` call THEN the system SHALL keep the persistent storage entry alive so that subsequent reads continue to succeed

### Unchanged Behavior (Regression Prevention)

3.1 WHEN `get_result` is called for a `match_id` that has no stored result THEN the system SHALL CONTINUE TO return `Error::ResultNotFound`
3.2 WHEN `submit_result` is called for a valid match THEN the system SHALL CONTINUE TO store the result and extend its TTL as before
3.3 WHEN `get_result` returns successfully THEN the system SHALL CONTINUE TO return the correct `ResultEntry` (game_id and result unchanged)
