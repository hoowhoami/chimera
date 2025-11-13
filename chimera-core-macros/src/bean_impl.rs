use proc_macro::TokenStream;

pub(crate) fn bean_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 目前只是标记，不做特殊处理
    // 后续可以扩展为自动注册等功能
    item
}
