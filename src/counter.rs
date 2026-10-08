// SPDX-FileCopyrightText: 2026 tok contributors
//
// SPDX-License-Identifier: MPL-2.0

//! Token counting: every tokenizer tok knows, behind one `Counter`.

#![allow(dead_code)]

/// Count BPE tokens in `text`, special tokens included as ordinary text.
pub(crate) fn count_tokens(bpe: &tiktoken_rs::CoreBPE, text: &str) -> usize {
    bpe.encode_with_special_tokens(text).len()
}

/// The default encoding: cl100k_base, compiled into the binary.
pub(crate) fn default_bpe() -> tiktoken_rs::CoreBPE {
    tiktoken_rs::cl100k_base().expect("cl100k_base is compiled in")
}
