//! Static analysis of weighted authorities (multisig) before they reach the chain.
//!
//! A Graphene authority is a threshold over weighted members. Some drafts are arithmetically
//! impossible — an `account_update` carrying one would brick the account. Others work but carry
//! operational risk, such as a threshold that stops clearing the moment a single member loses
//! access. This module answers both questions without touching the network, and enumerates the
//! minimal signer sets a UI can show.
//!
//! Findings come back as typed [`AuthorityIssue`] values rather than sentences, so callers render
//! them in their own wording and language.

/// A weighted authority member: an account (`alice`, `1.2.7`) or a public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedMember {
    /// Account name, object id or public key — whatever identifies the member to the caller.
    pub id: String,
    /// The member's vote weight.
    pub weight: u16,
}

impl WeightedMember {
    pub fn new(id: impl Into<String>, weight: u16) -> Self {
        Self {
            id: id.into(),
            weight,
        }
    }
}

/// Something worth telling the user about a draft authority.
///
/// [`AuthorityIssue::is_error`] separates drafts that must not be broadcast from those that are
/// merely risky.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityIssue {
    /// A zero threshold is satisfied without any signature at all.
    ZeroThreshold,
    /// An authority with no members can never be satisfied.
    NoMembers,
    /// The same member appears more than once.
    DuplicateMember { id: String },
    /// A zero-weight member contributes nothing when signing.
    ZeroWeightMember { id: String },
    /// Even every member signing together does not clear the threshold.
    ThresholdUnreachable { total_weight: u64, threshold: u32 },
    /// Losing this one member drops the reachable weight below the threshold.
    LockoutOnMemberLoss {
        id: String,
        remaining_weight: u64,
        threshold: u32,
    },
}

impl AuthorityIssue {
    /// Whether this finding must block the draft rather than just warn about it.
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::ZeroThreshold
                | Self::NoMembers
                | Self::DuplicateMember { .. }
                | Self::ThresholdUnreachable { .. }
        )
    }
}

/// The verdict on a draft authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityAnalysis {
    /// Whether the threshold can be met at all and no blocking issue was found.
    pub reachable: bool,
    /// Sum of every member's weight.
    pub total_weight: u64,
    /// Minimal member combinations that clear the threshold — no member in a set is redundant.
    /// Empty when the draft is unusable or there are too many members to enumerate.
    pub minimal_signer_sets: Vec<Vec<String>>,
    /// Everything found, blocking and advisory alike, in discovery order.
    pub issues: Vec<AuthorityIssue>,
}

impl AuthorityAnalysis {
    /// Findings that must block the draft.
    pub fn errors(&self) -> impl Iterator<Item = &AuthorityIssue> {
        self.issues.iter().filter(|issue| issue.is_error())
    }

    /// Findings that are advisory only.
    pub fn warnings(&self) -> impl Iterator<Item = &AuthorityIssue> {
        self.issues.iter().filter(|issue| !issue.is_error())
    }

    /// Whether any blocking finding was recorded.
    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }
}

/// Beyond this many members, enumerating minimal signer sets (2^n subsets) stops paying off.
pub const MAX_MEMBERS_FOR_SIGNER_SETS: usize = 12;
/// Upper bound on returned minimal signer sets.
pub const MAX_SIGNER_SETS: usize = 16;

/// Analyse a draft authority: a threshold plus every weighted member.
///
/// Accounts and keys go in the same list — the protocol sums both by the same measure.
pub fn analyze_authority(threshold: u32, members: &[WeightedMember]) -> AuthorityAnalysis {
    let mut issues = Vec::new();

    if threshold == 0 {
        issues.push(AuthorityIssue::ZeroThreshold);
    }
    if members.is_empty() {
        issues.push(AuthorityIssue::NoMembers);
    }

    let mut seen = std::collections::HashSet::new();
    for member in members {
        if !seen.insert(member.id.as_str()) {
            issues.push(AuthorityIssue::DuplicateMember {
                id: member.id.clone(),
            });
        }
        if member.weight == 0 {
            issues.push(AuthorityIssue::ZeroWeightMember {
                id: member.id.clone(),
            });
        }
    }

    let total_weight: u64 = members.iter().map(|m| u64::from(m.weight)).sum();
    if total_weight < u64::from(threshold) {
        issues.push(AuthorityIssue::ThresholdUnreachable {
            total_weight,
            threshold,
        });
    }

    let reachable = !issues.iter().any(AuthorityIssue::is_error) && threshold > 0;

    if reachable && members.len() > 1 {
        for member in members {
            let remaining_weight = total_weight - u64::from(member.weight);
            if remaining_weight < u64::from(threshold) {
                issues.push(AuthorityIssue::LockoutOnMemberLoss {
                    id: member.id.clone(),
                    remaining_weight,
                    threshold,
                });
            }
        }
    }

    let minimal_signer_sets = if reachable && members.len() <= MAX_MEMBERS_FOR_SIGNER_SETS {
        minimal_signer_sets(threshold, members)
    } else {
        Vec::new()
    };

    AuthorityAnalysis {
        reachable,
        total_weight,
        minimal_signer_sets,
        issues,
    }
}

/// Every minimal combination clearing `threshold`: smallest teams first, then by member order.
fn minimal_signer_sets(threshold: u32, members: &[WeightedMember]) -> Vec<Vec<String>> {
    let n = members.len();
    let mut satisfying: Vec<u32> = Vec::new();
    for mask in 1u32..(1 << n) {
        let weight: u64 = (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| u64::from(members[i].weight))
            .sum();
        if weight >= u64::from(threshold) {
            satisfying.push(mask);
        }
    }

    // Minimal = no proper subset of it already suffices.
    let mut minimal: Vec<u32> = satisfying
        .iter()
        .copied()
        .filter(|&mask| {
            !satisfying
                .iter()
                .any(|&other| other != mask && other & mask == other)
        })
        .collect();
    minimal.sort_by_key(|mask| (mask.count_ones(), *mask));

    minimal
        .into_iter()
        .take(MAX_SIGNER_SETS)
        .map(|mask| {
            (0..n)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| members[i].id.clone())
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(id: &str, weight: u16) -> WeightedMember {
        WeightedMember::new(id, weight)
    }

    #[test]
    fn single_key_is_reachable_without_warnings() {
        let analysis = analyze_authority(1, &[member("anna", 1)]);
        assert!(analysis.reachable);
        assert!(analysis.issues.is_empty());
        assert_eq!(analysis.minimal_signer_sets, vec![vec!["anna".to_string()]]);
    }

    #[test]
    fn two_of_three_enumerates_pairs() {
        let analysis = analyze_authority(
            2,
            &[
                member("anna", 1),
                member("tomasz", 1),
                member("platforma", 1),
            ],
        );
        assert!(analysis.reachable);
        assert_eq!(analysis.minimal_signer_sets.len(), 3);
        assert!(analysis.minimal_signer_sets.iter().all(|set| set.len() == 2));
    }

    #[test]
    fn two_of_two_warns_about_lockout() {
        let analysis = analyze_authority(2, &[member("anna", 1), member("tomasz", 1)]);
        assert!(analysis.reachable);
        assert_eq!(analysis.warnings().count(), 2);
        assert!(
            analysis
                .warnings()
                .all(|issue| matches!(issue, AuthorityIssue::LockoutOnMemberLoss { .. }))
        );
    }

    #[test]
    fn unreachable_threshold_is_an_error() {
        let analysis = analyze_authority(5, &[member("anna", 1), member("tomasz", 1)]);
        assert!(!analysis.reachable);
        assert!(analysis.errors().any(|issue| matches!(
            issue,
            AuthorityIssue::ThresholdUnreachable {
                total_weight: 2,
                threshold: 5
            }
        )));
        assert!(analysis.minimal_signer_sets.is_empty());
    }

    #[test]
    fn zero_threshold_and_duplicates_are_flagged() {
        let analysis = analyze_authority(0, &[member("anna", 1), member("anna", 1)]);
        assert!(!analysis.reachable);
        assert!(
            analysis
                .errors()
                .any(|issue| matches!(issue, AuthorityIssue::ZeroThreshold))
        );
        assert!(
            analysis
                .errors()
                .any(|issue| matches!(issue, AuthorityIssue::DuplicateMember { id } if id == "anna"))
        );
    }

    #[test]
    fn asymmetric_weights_prefer_heavy_member() {
        // anna(2) clears the threshold alone; tomasz+platforma only together.
        let analysis = analyze_authority(
            2,
            &[
                member("anna", 2),
                member("tomasz", 1),
                member("platforma", 1),
            ],
        );
        assert!(analysis.reachable);
        assert!(
            analysis
                .minimal_signer_sets
                .contains(&vec!["anna".to_string()])
        );
        assert!(
            analysis
                .minimal_signer_sets
                .contains(&vec!["tomasz".to_string(), "platforma".to_string()])
        );
    }

    #[test]
    fn zero_weight_member_warns() {
        let analysis = analyze_authority(1, &[member("anna", 1), member("obserwator", 0)]);
        assert!(analysis.reachable);
        assert!(analysis.warnings().any(
            |issue| matches!(issue, AuthorityIssue::ZeroWeightMember { id } if id == "obserwator")
        ));
    }

    #[test]
    fn empty_authority_is_an_error() {
        let analysis = analyze_authority(1, &[]);
        assert!(!analysis.reachable);
        assert!(
            analysis
                .errors()
                .any(|issue| matches!(issue, AuthorityIssue::NoMembers))
        );
    }
}
