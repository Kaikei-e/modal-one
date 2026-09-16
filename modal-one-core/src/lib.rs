#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Wasm ランタイム値 (MVP: i32/i64/f32/f64)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
}

/// 関数シグネチャ（バイナリ表現に合わせ resultsも Vec）
#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

/// decodeされた1関数。
#[derive(Debug, Clone)]
pub struct Function {
    pub type_index: u32,
    pub code: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub enum ExportKind {
    Func(u32),
    // TODO: Global, Table, Memory,
}

#[derive(Debug, Clone)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug)]
pub struct Module {
    pub types: Vec<FuncType>,
    pub functions: Vec<Function>,
    pub exports: Vec<Export>,
}

#[derive(Debug)]
pub struct ValidatedModule {
    pub module: Module,
    // side_table: ... 制御フロー実装。 TODO
}

#[derive(Debug)]
pub enum DecodeError {
    UnexpectedEof,
    InvalidMagic,
    InvalidVersion,
}

#[derive(Debug)]
pub enum ValidateError {
    TypeMismatch,
    UnknownExport,
}

#[derive(Debug)]
pub enum Trap {
    UnreachableExecuted,
}

pub fn decode(bytes: &[u8]) -> Result<Module, DecodeError> {
    todo!()
}

pub fn validate(module: Module) -> Result<ValidatedModule, ValidateError>{
    todo!()
}

pub fn invoke(
    module: &ValidatedModule,
    export_name: &str,
    args: &[Value],
) -> Result<Value, Trap>{
    todo!()
}