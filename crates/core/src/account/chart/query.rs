//! Reading the account tree.
//!
//! Every method here takes `&self`, so none of them can break an invariant.
//! The rules that hold the tree together live in the [parent module](super).

use super::{Account, AccountError, AccountId, AccountType, ChartOfAccounts, RootAccounts};

impl ChartOfAccounts {
    /// Returns the account with `id`, if the chart has one.
    #[must_use]
    pub fn get(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(&id)
    }

    /// Returns the direct children of an account.
    ///
    /// Only immediate children are yielded, not the whole subtree, and a leaf
    /// account yields nothing.
    ///
    /// The order is arbitrary. Sort before displaying, and compare as a set in
    /// tests.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`.
    pub fn children_of(
        &self,
        id: AccountId,
    ) -> Result<impl Iterator<Item = &Account>, AccountError> {
        self.get(id).ok_or(AccountError::NotFound { id })?;
        Ok(self.accounts.values().filter(move |a| a.parent_id() == Some(id)))
    }

    /// Returns the ids of the five roots, and which element each one carries.
    #[must_use]
    pub fn roots(&self) -> RootAccounts {
        self.roots
    }

    /// Returns true if `id` is one of the five roots.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`. An
    ///   unknown id is not reported as "not a root".
    pub fn is_root(&self, id: AccountId) -> Result<bool, AccountError> {
        let account = self.get(id).ok_or(AccountError::NotFound { id })?;
        Ok(account.is_root())
    }

    /// Returns the root account that `id` descends from.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`.
    /// - [`MaxDepthExceeded`](AccountError::MaxDepthExceeded) if the chart is
    ///   corrupt: the walk ran past the chart's internal depth guard.
    /// - [`Orphaned`](AccountError::Orphaned) if an account names a parent that
    ///   is not in the chart.
    pub fn root_of(&self, id: AccountId) -> Result<&Account, AccountError> {
        self.walk(id, |_| {})
    }

    /// Returns the element (or type) an account reports under.
    ///
    /// Derived by walking to the root, not stored, making a child contradicting
    /// its parent unrepresentable.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`.
    /// - [`NotARoot`](AccountError::NotARoot) if the account's root is not one
    ///   of the five. Unreachable in a well-formed chart, since only
    ///   construction makes a parentless account.
    /// - [`MaxDepthExceeded`](AccountError::MaxDepthExceeded) if the chart is
    ///   corrupt.
    /// - [`Orphaned`](AccountError::Orphaned) if an account names a parent that
    ///   is not in the chart.
    pub fn type_of(&self, id: AccountId) -> Result<AccountType, AccountError> {
        let root = self.root_of(id)?;
        self.roots.type_of(root.id())
    }

    /// Returns how far below its root an account sits. A root is level 0.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`.
    /// - [`MaxDepthExceeded`](AccountError::MaxDepthExceeded) if the chart is
    ///   corrupt.
    /// - [`Orphaned`](AccountError::Orphaned) if an account names a parent that
    ///   is not in the chart.
    pub fn level_of(&self, id: AccountId) -> Result<usize, AccountError> {
        let mut level = 0;
        self.walk(id, |_| level += 1)?;
        Ok(level - 1)
    }

    /// Returns the names from the root down to the account (root comes first).
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if no account has `id`.
    /// - [`MaxDepthExceeded`](AccountError::MaxDepthExceeded) if the chart is
    ///   corrupt.
    /// - [`Orphaned`](AccountError::Orphaned) if an account names a parent that
    ///   is not in the chart.
    pub fn path_of(&self, id: AccountId) -> Result<Vec<String>, AccountError> {
        let mut names = Vec::with_capacity(Self::EXPECTED_PATH_LENGTH);
        self.walk(id, |a| names.push(a.name().to_string()))?;
        names.reverse();
        Ok(names)
    }

    /// Walks from `id` to its root, handing each account to `f` on the way, and
    /// returns the root.
    ///
    /// # Errors
    /// - [`NotFound`](AccountError::NotFound) if `id`, or any ancestor of it,
    ///   is missing.
    /// - [`MaxDepthExceeded`](AccountError::MaxDepthExceeded) past
    ///   [`MAX_TREE_DEPTH`](Self::MAX_TREE_DEPTH) steps, which means the chart
    ///   is corrupt.
    /// - [`Orphaned`](AccountError::Orphaned) if an account names a parent that
    ///   is not in the chart.
    fn walk(&self, id: AccountId, mut f: impl FnMut(&Account)) -> Result<&Account, AccountError> {
        let mut account = self.get(id).ok_or(AccountError::NotFound { id })?;
        let mut depth = 0;
        loop {
            f(account);
            let Some(parent_id) = account.parent_id() else {
                return Ok(account);
            };
            depth += 1;
            if depth > Self::MAX_TREE_DEPTH {
                return Err(AccountError::MaxDepthExceeded { id, max: Self::MAX_TREE_DEPTH });
            }
            account = self
                .get(parent_id)
                .ok_or(AccountError::Orphaned { id: account.id(), parent_id })?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_support::{chart, id};

    /// Adds a chain of `depth` accounts under the expense root, returning the
    /// deepest. The deepest sits at level `depth`.
    fn chain(coa: &mut ChartOfAccounts, depth: usize) -> AccountId {
        let mut parent = coa.roots().expense();
        for i in 0..depth {
            let child = id(100 + i as u64);
            coa.add(child, &format!("Level {i}"), parent).expect("each add must succeed");
            parent = child;
        }
        parent
    }

    /// A three-level chart: Assets > Current > Bank, plus a Cash sibling of
    /// Current. Returns the chart.
    fn nested() -> ChartOfAccounts {
        let mut coa = chart();
        let asset = coa.roots().asset();
        coa.add(id(10), "Current", asset).expect("Current must add");
        coa.add(id(11), "Cash", asset).expect("Cash must add");
        coa.add(id(12), "Bank", id(10)).expect("Bank must add");
        coa
    }

    #[test]
    fn test_get_finds_only_what_is_there() {
        let coa = nested();
        assert_eq!(coa.get(id(12)).map(Account::name), Some("Bank"));
        assert!(coa.get(id(99)).is_none());
    }

    #[test]
    fn test_children_of_yields_direct_children_only() {
        let coa = nested();
        let asset = coa.roots().asset();

        let mut names: Vec<&str> = coa.children_of(asset).unwrap().map(Account::name).collect();
        names.sort_unstable();
        assert_eq!(names, ["Cash", "Current"], "Bank is a grandchild, not a child");

        assert_eq!(coa.children_of(id(10)).unwrap().count(), 1);
        assert_eq!(coa.children_of(id(12)).unwrap().count(), 0, "a leaf has no children");
    }

    #[test]
    fn test_children_of_rejects_an_unknown_id() {
        let coa = nested();
        let ghost = id(99);
        let err = coa.children_of(ghost).err().expect("id 99 is not in the chart");
        assert!(matches!(err, AccountError::NotFound { id } if id == ghost), "got {err}");
    }

    #[test]
    fn test_roots_reports_the_five_seeded_ids() {
        let coa = chart();
        let roots = coa.roots();
        for id in
            [roots.asset(), roots.liability(), roots.equity(), roots.income(), roots.expense()]
        {
            assert!(coa.get(id).is_some(), "{id} must be a real account");
            assert!(coa.is_root(id).unwrap());
        }
    }

    #[test]
    fn test_is_root_separates_missing_from_not_a_root() {
        let coa = nested();
        assert!(coa.is_root(coa.roots().asset()).unwrap());
        assert!(!coa.is_root(id(12)).unwrap());

        let err = coa.is_root(id(99)).unwrap_err();
        assert!(matches!(err, AccountError::NotFound { .. }), "got {err}");
    }

    #[test]
    fn test_root_of_climbs_to_the_top() {
        let coa = nested();
        let asset = coa.roots().asset();
        assert_eq!(coa.root_of(id(12)).unwrap().id(), asset, "from a grandchild");
        assert_eq!(coa.root_of(id(10)).unwrap().id(), asset, "from a child");
        assert_eq!(coa.root_of(asset).unwrap().id(), asset, "a root is its own root");
    }

    #[test]
    fn test_type_of_reports_the_root_element() {
        let mut coa = chart();
        let roots = coa.roots();
        let cases = [
            (roots.asset(), AccountType::Asset),
            (roots.liability(), AccountType::Liability),
            (roots.equity(), AccountType::Equity),
            (roots.income(), AccountType::Income),
            (roots.expense(), AccountType::Expense),
        ];
        for (index, (root, element)) in cases.into_iter().enumerate() {
            let child = id(10 + index as u64);
            coa.add(child, "Child", root).expect("each add must succeed");
            assert_eq!(coa.type_of(child).unwrap(), element);
            assert_eq!(coa.type_of(root).unwrap(), element);
        }
    }

    #[test]
    fn test_level_of_counts_from_the_root() {
        let coa = nested();
        assert_eq!(coa.level_of(coa.roots().asset()).unwrap(), 0);
        assert_eq!(coa.level_of(id(10)).unwrap(), 1);
        assert_eq!(coa.level_of(id(12)).unwrap(), 2);
    }

    #[test]
    fn test_path_of_reads_root_first() {
        let coa = nested();
        assert_eq!(coa.path_of(id(12)).unwrap(), ["Assets", "Current", "Bank"]);
        assert_eq!(coa.path_of(coa.roots().asset()).unwrap(), ["Assets"]);
    }

    #[test]
    fn test_tree_queries_reject_an_unknown_id() {
        let coa = nested();
        let ghost = id(99);
        assert!(matches!(coa.root_of(ghost), Err(AccountError::NotFound { .. })));
        assert!(matches!(coa.type_of(ghost), Err(AccountError::NotFound { .. })));
        assert!(matches!(coa.level_of(ghost), Err(AccountError::NotFound { .. })));
        assert!(matches!(coa.path_of(ghost), Err(AccountError::NotFound { .. })));
    }

    /// The depth guard is a corruption circuit breaker, so the boundary has to
    /// sit above anything a legitimate chart can build, and has to actually
    /// fire one step past it.
    #[test]
    fn test_the_depth_guard_fires_exactly_one_step_past_the_limit() {
        let mut coa = chart();
        let deepest = chain(&mut coa, ChartOfAccounts::MAX_TREE_DEPTH);
        assert_eq!(coa.level_of(deepest).unwrap(), ChartOfAccounts::MAX_TREE_DEPTH);

        // One more level, and the walk from the bottom refuses.
        let over = id(100 + ChartOfAccounts::MAX_TREE_DEPTH as u64);
        coa.add(over, "One too deep", deepest).unwrap();
        let err = coa.level_of(over).unwrap_err();
        assert!(
            matches!(err, AccountError::MaxDepthExceeded { max, .. }
                if max == ChartOfAccounts::MAX_TREE_DEPTH),
            "got {err}"
        );
    }

    /// Unreachable through the public API. Reached here by severing the chain
    /// by hand, because the variant exists precisely to name the account that
    /// broke it, and nothing else pins which end of the edge that is.
    #[test]
    fn test_a_severed_chain_names_the_account_holding_the_dangling_parent() {
        let mut coa = nested(); // Assets > Current(10) > Bank(12)
        coa.accounts.remove(&id(10));

        let err = coa.path_of(id(12)).unwrap_err();
        assert!(
            matches!(err, AccountError::Orphaned { id: got, .. } if got == id(12)),
            "the child holding the dangling id must be named, got {err}"
        );
    }
}
