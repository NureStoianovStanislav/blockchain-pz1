use core::fmt;

use anyhow::Context;
use base64::prelude::*;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

fn main() -> anyhow::Result<()> {
    let mut blockchain = Blockchain::new(3);
    blockchain.add("asdfadfas".into());
    blockchain.add("foo".into());
    blockchain.add("bar".into());
    blockchain.add("baz".into());
    blockchain.add("egg".into());
    blockchain.add("balls".into());
    // blockchain.tail.as_mut().unwrap().prev.as_mut().unwrap().1 = "invalid hash".to_owned();
    blockchain.verify_chain()?;
    println!("{blockchain:?}");
    Ok(())
}

pub struct Block {
    prev: Option<(Box<Self>, String)>,
    timestamp: DateTime<Utc>,
    transaction: String,
    nonce: u64,
}

struct Blockchain {
    tail: Option<Box<Block>>,
    difficulty: usize,
}

impl Blockchain {
    fn new(difficulty: usize) -> Self {
        Self {
            tail: None,
            difficulty,
        }
    }

    fn add(&mut self, transaction: String) {
        let prev = self.tail.take().map(|block| {
            let hash = self.hash_block(&block);
            (block, hash)
        });
        let new_block = Block {
            prev,
            timestamp: Utc::now(),
            transaction,
            nonce: 0,
        };
        self.tail = Some(Box::new(self.mine_block(new_block)));
    }

    fn mine_block(&self, mut block: Block) -> Block {
        block.nonce = Default::default();
        while self.check_hash(&self.hash_block(&block)).is_err() {
            block.nonce += 1;
        }
        block
    }

    fn hash_block(&self, block: &Block) -> String {
        let timestamp = block.timestamp.to_string();
        let nonce = block.nonce.to_le_bytes();
        let mut hasher = Sha256::new();
        block.prev.as_ref().inspect(|block| hasher.update(&block.1));
        let digest = hasher
            .chain_update(timestamp)
            .chain_update(&block.transaction)
            .chain_update(nonce)
            .finalize();
        BASE64_STANDARD.encode(digest)
    }

    fn check_hash(&self, hash: &str) -> anyhow::Result<()> {
        if !hash.chars().take(self.difficulty).all(|c| c == '0') {
            anyhow::bail!("invalid hash: {hash:?}")
        }
        Ok(())
    }

    fn verify_chain(&self) -> anyhow::Result<()> {
        let mut blocks = self.iter();
        let tail = blocks.next();
        blocks
            .try_fold(tail, |next, prev| {
                let expected_hash = next
                    .and_then(|b| b.prev.as_ref().map(|(_, h)| h.as_str()))
                    .unwrap();
                match self.hash_block(prev) {
                    h if h != expected_hash => Err(anyhow::anyhow!(
                        "hash mismatch: specified {expected_hash:?} when expected {h:?}"
                    )),
                    h => self.check_hash(&h).map(|()| Some(prev)),
                }
                .with_context(|| format!("invalid block in chain: {next:?}"))
            })
            .map(|_| ())
    }

    fn iter(&self) -> impl Iterator<Item = &Block> {
        core::iter::successors(self.tail.as_ref(), |next| {
            next.prev.as_ref().map(|(prev, _)| prev)
        })
        .map(Box::as_ref)
    }
}

impl fmt::Debug for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut blocks = core::iter::successors(self.tail.as_ref(), |next| {
            next.prev.as_ref().map(|(prev, _)| prev)
        });
        blocks
            .next()
            .into_iter()
            .try_for_each(|block| write!(f, "{block:?}"))?;
        blocks.try_for_each(|block| write!(f, "\n^\n{block:?}"))
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>())
            .field(
                "prev",
                &format_args!("{:?}", self.prev.as_ref().map(|(_, hash)| hash)),
            )
            .field("timestamp", &format_args!("{:?}", self.timestamp))
            .field("transaction", &format_args!("{}", self.transaction))
            .finish()
    }
}
