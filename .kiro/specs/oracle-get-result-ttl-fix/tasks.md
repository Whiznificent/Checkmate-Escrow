# Implementation Plan

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - TTL Not Extended on Read
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate the bug exists
  - **Scoped PBT Approach**: Scope the property to the concrete failing case - submit result, advance ledgers, call get_result, verify TTL is NOT refreshed
  - Test that get_result does not extend TTL when reading an existing result entry (from Bug Condition in design)
  - The test assertions should match the Expected Behavior Properties from design (TTL should be reset to MATCH_TTL_LEDGERS after read)
  - Test cases:
    - Submit result for match_id on ledger 1000
    - Advance to ledger 2000 (consume 1000 ledgers of TTL)
    - Call get_result(match_id)
    - Assert TTL equals MATCH_TTL_LEDGERS (518,400) - this will FAIL on unfixed code where TTL is ~517,400
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (this is correct - it proves the bug exists)
  - Document counterexamples found (e.g., "TTL is 517,400 instead of 518,400 after get_result call")
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Non-Existent Results and Data Integrity
  - **IMPORTANT**: Follow observation-first methodology
  - Observe behavior on UNFIXED code for non-buggy inputs (cases where isBugCondition returns false)
  - Test cases:
    - Observe: get_result for a match_id with no stored result returns Error::ResultNotFound on unfixed code
    - Observe: get_result returns correct ResultEntry with unchanged game_id and result fields on unfixed code
    - Observe: submit_result extends TTL to MATCH_TTL_LEDGERS on unfixed code
    - Write property: for all match_ids without stored results, get_result returns Error::ResultNotFound
    - Write property: for all match_ids with stored results, get_result returns correct ResultEntry data (game_id and result unchanged)
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3_

- [-] 3. Fix for get_result TTL extension

  - [x] 3.1 Implement the fix in contracts/oracle/src/lib.rs
    - Locate the get_result function
    - Bind the retrieved entry to a local variable using the `?` operator to propagate Error::ResultNotFound on missing entries
    - Add extend_ttl call after successful retrieval: env.storage().persistent().extend_ttl(&DataKey::Result(match_id), MATCH_TTL_LEDGERS, MATCH_TTL_LEDGERS)
    - Place the extend_ttl call before returning Ok(entry)
    - Ensure the error path (non-existent results) remains unchanged - the `?` operator handles this
    - _Bug_Condition: isBugCondition(input) where env.storage().persistent().has(&DataKey::Result(input.match_id)) AND get_result_called(input.match_id) AND NOT ttl_extended_after_read(input.match_id)_
    - _Expected_Behavior: get_result SHALL extend TTL to MATCH_TTL_LEDGERS before returning the entry_
    - _Preservation: Non-existent results return Error::ResultNotFound unchanged; submit_result TTL behavior unchanged; returned ResultEntry data (game_id, result) unchanged_
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2, 3.3_

  - [-] 3.2 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** - TTL Extended on Read
    - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
    - The test from task 1 encodes the expected behavior
    - When this test passes, it confirms the expected behavior is satisfied
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed - TTL is now reset to MATCH_TTL_LEDGERS after get_result)
    - _Requirements: 2.1, 2.2_

  - [ ] 3.3 Verify preservation tests still pass
    - **Property 2: Preservation** - Non-Existent Results and Data Integrity
    - **IMPORTANT**: Re-run the SAME tests from task 2 - do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Verify non-existent results still return Error::ResultNotFound
    - Verify returned ResultEntry data (game_id, result) is unchanged
    - Verify submit_result TTL behavior is unchanged

- [ ] 4. Checkpoint - Ensure all tests pass
  - Run all tests (exploration + preservation)
  - Verify bug condition test passes (TTL extended on read)
  - Verify preservation tests pass (no regressions)
  - Ensure all tests pass, ask the user if questions arise
