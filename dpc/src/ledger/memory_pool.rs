// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use crate::prelude::*;

use anyhow::{anyhow, ensure, Result};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct MemoryPool<N: Network> {
    /// The pool of unconfirmed transactions.
    transactions: HashMap<N::TransactionID, Transaction<N>>,
    /// The list of unconfirmed serial numbers.
    serial_numbers: HashSet<N::SerialNumber>,
    /// The list of unconfirmed commitments.
    commitments: HashSet<N::Commitment>,
    /// The set of open requests.
    #[allow(dead_code)]
    requests: HashSet<Request<N>>,
}

impl<N: Network> MemoryPool<N> {
    /// Initializes a new instance of a memory pool.
    pub fn new() -> Self {
        Self {
            transactions: Default::default(),
            serial_numbers: Default::default(),
            commitments: Default::default(),
            requests: Default::default(),
        }
    }

    /// Returns `true` if the given transaction exists in the memory pool.
    pub fn contains_transaction(&self, transaction: &Transaction<N>) -> bool {
        self.transactions.contains_key(&transaction.transaction_id())
    }

    /// Returns the transactions in the memory pool.
    pub fn transactions(&self) -> Vec<Transaction<N>> {
        self.transactions.values().cloned().collect()
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub fn add_transaction(&mut self, transaction: &Transaction<N>) -> Result<()> {
        ensure!(transaction.is_valid(), "The unconfirmed transaction is invalid");

        // Ensure the transaction does not attempt to mint new value.
        ensure!(
            !transaction.value_balance().is_negative(),
            "The unconfirmed transaction is attempting to mint new value"
        );

        let transaction_id = transaction.transaction_id();
        ensure!(!self.transactions.contains_key(&transaction_id), "Transaction already exists in memory pool");

        for serial_number in transaction.serial_numbers() {
            ensure!(!self.serial_numbers.contains(serial_number), "Serial number already used in memory pool");
        }

        for commitment in transaction.commitments() {
            ensure!(!self.commitments.contains(commitment), "Commitment already used in memory pool");
        }

        // Add the transaction to the memory pool. This code section executes atomically.
        {
            let mut memory_pool = self.clone();

            memory_pool.transactions.insert(transaction_id, transaction.clone());

            memory_pool.serial_numbers.extend(transaction.serial_numbers());
            memory_pool.commitments.extend(transaction.commitments());

            *self = memory_pool;
        }

        Ok(())
    }

    /// Clear a transaction (and associated state) from the memory pool.
    pub fn remove_transaction(&mut self, transaction: &Transaction<N>) {
        // This code section executes atomically.

        let mut memory_pool = self.clone();

        memory_pool.transactions.remove(&transaction.transaction_id());
        for serial_number in transaction.serial_numbers() {
            memory_pool.serial_numbers.remove(serial_number);
        }
        for commitment in transaction.commitments() {
            memory_pool.commitments.remove(commitment);
        }

        *self = memory_pool;
    }

    /// Clear a list of transactions (and associated state) from the memory pool.
    pub fn remove_transactions(&mut self, transactions: &[Transaction<N>]) {
        // This code section executes atomically.

        let mut memory_pool = self.clone();

        for transaction in transactions {
            memory_pool.transactions.remove(&transaction.transaction_id());
            for serial_number in transaction.serial_numbers() {
                memory_pool.serial_numbers.remove(serial_number);
            }
            for commitment in transaction.commitments() {
                memory_pool.commitments.remove(commitment);
            }
        }

        *self = memory_pool;
    }

    /// Clears all transactions (and associated state) from the memory pool.
    pub fn clear_all_transactions(&mut self) {
        self.transactions = Default::default();
        self.serial_numbers = Default::default();
        self.commitments = Default::default();
    }
}

impl<N: Network> Default for MemoryPool<N> {
    fn default() -> Self {
        Self::new()
    }
}
