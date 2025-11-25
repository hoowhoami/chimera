use proc_macro::TokenStream;

/// Configuration derive macro implementation
/// Configuration is just a Component with a special semantic meaning
pub(crate) fn derive_configuration_impl(input: TokenStream) -> TokenStream {
    // Reuse Component implementation since Configuration is a Component
    crate::component_impl::derive_component_impl(input)
}
