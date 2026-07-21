use etas_core::id_type;

id_type!(OperandId);
id_type!(UserId);
id_type!(UseId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operand<O> {
    pub id: OperandId,
    pub value: O,
    first_use: Option<UseId>,
}

impl<O> Operand<O> {
    pub fn first_use(&self) -> Option<UseId> {
        self.first_use
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User<U> {
    pub id: UserId,
    pub value: U,
    operand_slots: Vec<Option<UseId>>,
}

impl<U> User<U> {
    pub fn operand_slot_count(&self) -> usize {
        self.operand_slots.len()
    }

    pub fn use_at(&self, index: usize) -> Option<UseId> {
        self.operand_slots.get(index).copied().flatten()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Use {
    pub id: UseId,
    pub user: UserId,
    pub operand_index: usize,
    pub operand: OperandId,
    prev_use: Option<UseId>,
    next_use: Option<UseId>,
}

impl Use {
    pub fn prev_use(&self) -> Option<UseId> {
        self.prev_use
    }

    pub fn next_use(&self) -> Option<UseId> {
        self.next_use
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseList<U, O> {
    users: Vec<Option<User<U>>>,
    operands: Vec<Option<Operand<O>>>,
    uses: Vec<Option<Use>>,
    live_users: usize,
    live_operands: usize,
    live_uses: usize,
}

impl<U, O> Default for UseList<U, O> {
    fn default() -> Self {
        Self {
            users: Vec::new(),
            operands: Vec::new(),
            uses: Vec::new(),
            live_users: 0,
            live_operands: 0,
            live_uses: 0,
        }
    }
}

impl<U, O> UseList<U, O> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_operand(&mut self, value: O) -> OperandId {
        let id = OperandId(self.operands.len().min(u32::MAX as usize) as u32);
        self.operands.push(Some(Operand {
            id,
            value,
            first_use: None,
        }));
        self.live_operands += 1;
        id
    }

    pub fn add_user(&mut self, value: U) -> UserId {
        let id = UserId(self.users.len().min(u32::MAX as usize) as u32);
        self.users.push(Some(User {
            id,
            value,
            operand_slots: Vec::new(),
        }));
        self.live_users += 1;
        id
    }

    pub fn append_use(&mut self, user: UserId, operand: OperandId) -> Option<UseId> {
        if !self.contains_user(user) || !self.contains_operand(operand) {
            return None;
        }

        let id = UseId(self.uses.len().min(u32::MAX as usize) as u32);
        let operand_index = self.user(user)?.operand_slot_count();
        let next_use = self.operand(operand)?.first_use;

        self.uses.push(Some(Use {
            id,
            user,
            operand_index,
            operand,
            prev_use: None,
            next_use,
        }));

        if let Some(next_use) = next_use {
            self.use_mut(next_use)?.prev_use = Some(id);
        }
        self.operand_mut(operand)?.first_use = Some(id);
        self.user_mut(user)?.operand_slots.push(Some(id));
        self.live_uses += 1;
        Some(id)
    }

    pub fn set_use_operand(&mut self, use_id: UseId, operand: OperandId) -> Option<OperandId> {
        if !self.contains_use(use_id) || !self.contains_operand(operand) {
            return None;
        }

        let old_operand = self.use_(use_id)?.operand;
        if old_operand == operand {
            return Some(old_operand);
        }

        self.detach_use_from_operand(use_id)?;
        self.attach_use_to_operand(use_id, operand)?;
        Some(old_operand)
    }

    pub fn set_user_operand(
        &mut self,
        user: UserId,
        operand_index: usize,
        operand: OperandId,
    ) -> Option<OperandId> {
        let use_id = self.user(user)?.use_at(operand_index)?;
        self.set_use_operand(use_id, operand)
    }

    pub fn replace_all_uses_with(&mut self, from: OperandId, to: OperandId) -> Option<usize> {
        if from == to {
            return Some(0);
        }
        if !self.contains_operand(from) || !self.contains_operand(to) {
            return None;
        }

        let mut replaced = 0;
        while let Some(use_id) = self.operand(from)?.first_use {
            self.set_use_operand(use_id, to)?;
            replaced += 1;
        }
        Some(replaced)
    }

    pub fn remove_use(&mut self, use_id: UseId) -> Option<Use> {
        let (user, operand_index) = {
            let use_ref = self.use_(use_id)?;
            (use_ref.user, use_ref.operand_index)
        };
        self.detach_use_from_operand(use_id)?;
        if let Some(slot) = self.user_mut(user)?.operand_slots.get_mut(operand_index) {
            *slot = None;
        }
        self.live_uses -= 1;
        self.uses.get_mut(use_id.index())?.take()
    }

    pub fn remove_user(&mut self, user: UserId) -> Option<User<U>> {
        let uses = self.user_uses(user)?;
        for use_id in uses {
            self.remove_use(use_id);
        }
        let removed = self.users.get_mut(user.index())?.take()?;
        self.live_users -= 1;
        Some(removed)
    }

    pub fn remove_operand(&mut self, operand: OperandId) -> Option<Operand<O>> {
        if self.operand(operand)?.first_use.is_some() {
            return None;
        }
        let removed = self.operands.get_mut(operand.index())?.take()?;
        self.live_operands -= 1;
        Some(removed)
    }

    pub fn user(&self, user: UserId) -> Option<&User<U>> {
        self.users.get(user.index())?.as_ref()
    }

    pub fn user_mut(&mut self, user: UserId) -> Option<&mut User<U>> {
        self.users.get_mut(user.index())?.as_mut()
    }

    pub fn user_value(&self, user: UserId) -> Option<&U> {
        self.user(user).map(|user| &user.value)
    }

    pub fn user_value_mut(&mut self, user: UserId) -> Option<&mut U> {
        self.user_mut(user).map(|user| &mut user.value)
    }

    pub fn operand(&self, operand: OperandId) -> Option<&Operand<O>> {
        self.operands.get(operand.index())?.as_ref()
    }

    pub fn operand_mut(&mut self, operand: OperandId) -> Option<&mut Operand<O>> {
        self.operands.get_mut(operand.index())?.as_mut()
    }

    pub fn operand_value(&self, operand: OperandId) -> Option<&O> {
        self.operand(operand).map(|operand| &operand.value)
    }

    pub fn operand_value_mut(&mut self, operand: OperandId) -> Option<&mut O> {
        self.operand_mut(operand).map(|operand| &mut operand.value)
    }

    pub fn use_(&self, use_id: UseId) -> Option<&Use> {
        self.uses.get(use_id.index())?.as_ref()
    }

    pub fn use_mut(&mut self, use_id: UseId) -> Option<&mut Use> {
        self.uses.get_mut(use_id.index())?.as_mut()
    }

    pub fn operand_uses(&self, operand: OperandId) -> Option<Vec<UseId>> {
        let mut result = Vec::new();
        let mut current = self.operand(operand)?.first_use;
        while let Some(use_id) = current {
            let use_ref = self.use_(use_id)?;
            result.push(use_id);
            current = use_ref.next_use;
        }
        Some(result)
    }

    pub fn user_uses(&self, user: UserId) -> Option<Vec<UseId>> {
        Some(
            self.user(user)?
                .operand_slots
                .iter()
                .copied()
                .flatten()
                .collect(),
        )
    }

    pub fn user_operands(&self, user: UserId) -> Option<Vec<OperandId>> {
        self.user_uses(user)?
            .into_iter()
            .map(|use_id| self.use_(use_id).map(|use_ref| use_ref.operand))
            .collect()
    }

    pub fn contains_user(&self, user: UserId) -> bool {
        self.users.get(user.index()).is_some_and(Option::is_some)
    }

    pub fn contains_operand(&self, operand: OperandId) -> bool {
        self.operands
            .get(operand.index())
            .is_some_and(Option::is_some)
    }

    pub fn contains_use(&self, use_id: UseId) -> bool {
        self.uses.get(use_id.index()).is_some_and(Option::is_some)
    }

    pub fn user_count(&self) -> usize {
        self.live_users
    }

    pub fn operand_count(&self) -> usize {
        self.live_operands
    }

    pub fn use_count(&self) -> usize {
        self.live_uses
    }

    fn detach_use_from_operand(&mut self, use_id: UseId) -> Option<OperandId> {
        let (operand, prev_use, next_use) = {
            let use_ref = self.use_(use_id)?;
            (use_ref.operand, use_ref.prev_use, use_ref.next_use)
        };

        if let Some(prev_use) = prev_use {
            self.use_mut(prev_use)?.next_use = next_use;
        } else {
            self.operand_mut(operand)?.first_use = next_use;
        }

        if let Some(next_use) = next_use {
            self.use_mut(next_use)?.prev_use = prev_use;
        }

        let use_ref = self.use_mut(use_id)?;
        use_ref.prev_use = None;
        use_ref.next_use = None;
        Some(operand)
    }

    fn attach_use_to_operand(&mut self, use_id: UseId, operand: OperandId) -> Option<()> {
        let first_use = self.operand(operand)?.first_use;
        {
            let use_ref = self.use_mut(use_id)?;
            use_ref.operand = operand;
            use_ref.prev_use = None;
            use_ref.next_use = first_use;
        }
        if let Some(first_use) = first_use {
            self.use_mut(first_use)?.prev_use = Some(use_id);
        }
        self.operand_mut(operand)?.first_use = Some(use_id);
        Some(())
    }
}
