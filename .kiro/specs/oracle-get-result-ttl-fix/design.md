# Oracle get_result TTL Fix Design

## Overview

The `get_result` function in the oracle contract reads `ResultEntry` data from persistent storage but fails to extend its TTL. This causes entries to expire after `MATCH_TTL_LEDGERS` (approximately 30 days) without refresh, even when the entry is actively being read. The fix adds a single `extend_ttl` call after successfully retrieving the entry, ensuring that repeated reads keep the data alive for the escrow contract's payout flow.

## Glossary

- **Bug_Condition (C)**: The condition that triggers the bug - when `get_result` is called for an existing result entry
- **Property (P)**: The desired behavior when reading results - the TTL should be extended to prevent expiration
- **Preservation**: Existing behavior for non-existent results, submit_result TTL extension, and returned data correctness must remain unchanged
- **get_result**: The function in `contracts/oracle/src/lib.rs` that retrieves a stored match result from persistent storage
- **MATCH_TTL_LEDGERS**: The constant (518,400 ledgers ≈ 30 days) defining how long result entries remain in persistent storage
- **ResultEntry**: The stored data structure containing `game_id` and `result` for a match

## Bug Details

### Bug Condition

The bug manifests when `get_result` is called for a match that has a stored result entry. The function successfully retrieves and returns the entry but never calls `extend_ttl` on the persistent storage key, allowing the entry's TTL to count down with each ledger. After `MATCH_TTL_LEDGERS` ledgers pass since the last `submit_result` call, the entry expires and subsequent `get_result` calls return `Error::ResultNotFound`, breaking the escrow payout flow.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type (match_id: u64)
  OUTPUT: boolean
  
  RETURN env.storage().persistent().has(&DataKey::Result(input.match_id))
         AND get_result_called(input.match_id)
         AND NOT ttl_extended_after_read(input.match_id)
END FUNCTION
```

### Examples

- **Example 1**: Admin submits result for match 42 on ledger 1000. Escrow contract calls `get_result(42)` on ledger 1500. Entry is returned successfully but TTL is not refreshed. On ledger 519,401 (1000 + 518,400 + 1), the entry expires. Subsequent `get_result(42)` calls return `Error::ResultNotFound` even though the result was legitimately submitted.

- **Example 2**: Result submitted for match 7 on ledger 5000. Multiple `get_result(7)` calls occur on ledgers 5100, 5200, 5300. Each read succeeds but TTL continues counting down from the original submission. Entry expires on ledger 523,401 instead of being kept alive by the reads.

- **Example 3**: Result submitted for match 15 on ledger 10,000. Escrow contract attempts payout on ledger 528,401 (after TTL expiration). `get_result(15)` returns `Error::ResultNotFound`, causing payout to fail despite the result being validly submitted.

- **Edge case**: `get_result` called for match 999 that has no stored result. Function correctly returns `Error::ResultNotFound` without attempting TTL extension (expected behavior, not a bug).

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- `get_result` must continue to return `Error::ResultNotFound` for match IDs with no stored result
- `submit_result` must continue to store results and extend TTL exactly as before
- `get_result` must continue to return the correct `ResultEntry` with unchanged `game_id` and `result` fields

**Scope:**
All inputs that do NOT involve reading an existing result entry should be completely unaffected by this fix. This includes:
- Calls to `get_result` for non-existent match IDs (should still return `Error::ResultNotFound`)
- Calls to `submit_result` (TTL extension behavior unchanged)
- Calls to `has_result` and `has_result_admin` (no TTL extension expected)
- The structure and content of returned `ResultEntry` data

## Hypothesized Root Cause

Based on the bug description, the root cause is clear:

1. **Missing TTL Extension Call**: The `get_result` function retrieves the entry using `env.storage().persistent().get()` but never calls `env.storage().persistent().extend_ttl()` afterwards
   - The function returns immediately after retrieving the entry
   - No TTL management logic exists in the read path

2. **Asymmetric TTL Management**: `submit_result` correctly extends TTL when writing, but `get_result` does not extend TTL when reading
   - This creates an asymmetry where writes refresh the TTL but reads do not
   - For long-lived matches with infrequent submissions but frequent reads, this causes premature expiration

3. **Implicit Assumption**: The original implementation may have assumed that results would only be read shortly after submission, within the initial TTL window
   - This assumption breaks for matches with delayed payouts or multiple payout attempts

## Correctness Properties

Property 1: Bug Condition - TTL Extended on Read

_For any_ match_id where a result entry exists in persistent storage, the fixed get_result function SHALL extend the TTL of that entry by MATCH_TTL_LEDGERS before returning it, ensuring the entry remains accessible for future reads.

**Validates: Requirements 2.1, 2.2**

Property 2: Preservation - Non-Existent Results

_For any_ match_id that does NOT have a stored result entry, the fixed get_result function SHALL produce exactly the same behavior as the original function, returning Error::ResultNotFound without attempting TTL extension.

**Validates: Requirements 3.1**

Property 3: Preservation - Returned Data Correctness

_For any_ match_id with a stored result, the fixed get_result function SHALL return a ResultEntry with identical game_id and result fields as the original function, preserving data integrity.

**Validates: Requirements 3.3**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct:

**File**: `contracts/oracle/src/lib.rs`

**Function**: `get_result`

**Specific Changes**:
1. **Add TTL Extension After Retrieval**: Insert a call to `env.storage().persistent().extend_ttl()` immediately after successfully retrieving the entry
   - Use the same TTL parameters as `submit_result`: `(MATCH_TTL_LEDGERS, MATCH_TTL_LEDGERS)`
   - Place the call before returning the entry to ensure TTL is always extended on successful reads

2. **Maintain Error Path**: Ensure that if the entry does not exist, the function returns `Error::ResultNotFound` without attempting TTL extension
   - The `ok_or(Error::ResultNotFound)?` pattern already handles this correctly
   - No changes needed to the error path

3. **Preserve Return Value**: Continue returning the retrieved `ResultEntry` unchanged
   - No modifications to the entry data structure or content

**Current Implementation (Buggy):**
```rust
pub fn get_result(env: Env, match_id: u64) -> Result<ResultEntry, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Result(match_id))
        .ok_or(Error::ResultNotFound)
}
```

**Fixed Implementation:**
```rust
pub fn get_result(env: Env, match_id: u64) -> Result<ResultEntry, Error> {
    let entry = env
        .storage()
        .persistent()
        .get(&DataKey::Result(match_id))
        .ok_or(Error::ResultNotFound)?;
    env.storage().persistent().extend_ttl(
        &DataKey::Result(match_id),
        MATCH_TTL_LEDGERS,
        MATCH_TTL_LEDGERS,
    );
    Ok(entry)
}
```

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code by observing TTL decay without extension, then verify the fix correctly extends TTL on reads while preserving all existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm that reading a result does not extend its TTL on the unfixed code.

**Test Plan**: Write a test that submits a result, advances the ledger sequence to partially consume the TTL, calls `get_result`, then checks the TTL value. Run this test on the UNFIXED code to observe that the TTL is not refreshed.

**Test Cases**:
1. **TTL Not Extended on Read (Unfixed)**: Submit result on ledger 1000, advance to ledger 2000, call `get_result`, verify TTL is approximately 517,400 (original TTL minus 1000 ledgers consumed) rather than the full 518,400 (will fail on unfixed code - TTL not refreshed)

2. **Entry Expiration After TTL**: Submit result on ledger 1000, advance to ledger 518,401, call `get_result`, verify it returns `Error::ResultNotFound` (will fail on unfixed code - entry expired)

3. **Multiple Reads Without Extension**: Submit result, call `get_result` multiple times with ledger advances between calls, verify TTL continues to decay (will fail on unfixed code - no TTL refresh)

**Expected Counterexamples**:
- TTL value remains at (MATCH_TTL_LEDGERS - ledgers_elapsed) instead of being reset to MATCH_TTL_LEDGERS
- Possible causes: missing `extend_ttl` call in `get_result` function

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds (existing result entries), the fixed function extends the TTL.

**Pseudocode:**
```
FOR ALL match_id WHERE env.storage().persistent().has(&DataKey::Result(match_id)) DO
  initial_ttl := get_ttl(&DataKey::Result(match_id))
  advance_ledgers(1000)
  result := get_result_fixed(match_id)
  new_ttl := get_ttl(&DataKey::Result(match_id))
  ASSERT new_ttl = MATCH_TTL_LEDGERS
  ASSERT result is Ok(ResultEntry)
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold (non-existent results), the fixed function produces the same result as the original function.

**Pseudocode:**
```
FOR ALL match_id WHERE NOT env.storage().persistent().has(&DataKey::Result(match_id)) DO
  ASSERT get_result_original(match_id) = get_result_fixed(match_id)
  ASSERT both return Error::ResultNotFound
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many test cases automatically across the input domain (various match_id values)
- It catches edge cases that manual unit tests might miss (boundary values, large match_ids)
- It provides strong guarantees that behavior is unchanged for all non-existent results

**Test Plan**: Observe behavior on UNFIXED code first for non-existent match IDs, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Non-Existent Result Returns Error**: Observe that `get_result(999)` returns `Error::ResultNotFound` on unfixed code, then write test to verify this continues after fix

2. **Returned Data Unchanged**: Observe that `get_result` returns correct `game_id` and `result` fields on unfixed code, then write test to verify data integrity is preserved after fix

3. **submit_result TTL Extension Unchanged**: Observe that `submit_result` extends TTL to MATCH_TTL_LEDGERS on unfixed code, then write test to verify this behavior continues after fix

### Unit Tests

- Test that `get_result` extends TTL to MATCH_TTL_LEDGERS after reading an existing entry
- Test that `get_result` returns `Error::ResultNotFound` for non-existent match IDs without attempting TTL extension
- Test that returned `ResultEntry` contains correct `game_id` and `result` values
- Test that multiple consecutive `get_result` calls each extend the TTL
- Test that `submit_result` TTL extension behavior remains unchanged

### Property-Based Tests

- Generate random match_ids with stored results and verify `get_result` always extends TTL to MATCH_TTL_LEDGERS
- Generate random match_ids without stored results and verify `get_result` always returns `Error::ResultNotFound`
- Generate random ledger sequences and verify TTL is correctly extended regardless of timing
- Test that returned data integrity is preserved across many random scenarios

### Integration Tests

- Test full flow: submit result, advance ledgers significantly, call `get_result`, verify entry remains accessible
- Test escrow contract integration: submit result, escrow calls `get_result` for payout, verify success even after long delays
- Test repeated reads over extended time periods keep entry alive indefinitely
