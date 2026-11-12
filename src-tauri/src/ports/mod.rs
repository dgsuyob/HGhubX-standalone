//! 端口定义：业务层依赖这些能力，不依赖具体上游。

pub trait SourcePort {
    fn adapter(&self) -> &'static str;
    fn origin(&self) -> &str;
}

pub trait SessionPort {
    /// 当前源是否已登录（token 存在且 origin 匹配）。
    fn signed_in(&self) -> bool;
}
