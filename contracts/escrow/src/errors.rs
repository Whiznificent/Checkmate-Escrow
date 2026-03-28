use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    MatchNotFound = 1,
    AlreadyFunded = 2,
    NotFunded = 3,
    Unauthorized = 4,
    InvalidState = 5,
    AlreadyExists = 6,
    AlreadyInitialized = 7,
    Overflow = 8,
    ContractPaused = 9,
    InvalidAmount = 10,
    MatchCancelled = 11,
    MatchCompleted = 12,
    DuplicateGameId = 13,
    MatchNotExpired = 14,
    InvalidGameId = 15,
    /// The match_id was once valid but its storage entry has been evicted
    /// (TTL elapsed). The match existed but is no longer accessible on-chain.
    MatchStorageExpired = 16,
}
