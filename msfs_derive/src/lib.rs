use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, ExprLit, ExprPath, Fields, Lit, LitInt, LitStr, parse_macro_input,
    spanned::Spanned,
};

#[proc_macro_derive(VarStruct, attributes(var))]
pub fn derive_var_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_var_struct_impl(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarKindSel {
    A,
    L,
}

struct FieldSpec {
    ident: syn::Ident,
    name: String,
    unit: String,
    kind: VarKindSel,
    index: Option<u32>,
    target: Option<VarTargetSel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarTargetSel {
    UserAircraft,
    UserAvatar,
    UserCurrent,
}

fn derive_var_struct_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let input_span = input.span();
    let struct_ident = input.ident.clone();

    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(named) => named.named,
            _ => {
                return Err(syn::Error::new(
                    s.fields.span(),
                    "VarStruct can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                input_span,
                "VarStruct can only be derived for structs",
            ));
        }
    };

    let mut specs = Vec::<FieldSpec>::new();

    for field in fields {
        let field_span = field.span();
        let Some(ident) = field.ident.clone() else {
            continue;
        };

        // Currently only supports f64 fields.
        if !is_f64_type(&field.ty) {
            return Err(syn::Error::new(
                field.ty.span(),
                "VarStruct currently only supports fields of type f64",
            ));
        }

        let var_attr = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("var"))
            .ok_or_else(|| {
                syn::Error::new(
                    field_span,
                    "missing #[var(...)] attribute (expected at least name + unit)",
                )
            })?;

        let mut name: Option<String> = None;
        let mut unit: Option<String> = None;
        let mut kind: Option<VarKindSel> = None;
        let mut index: Option<u32> = None;
        let mut target: Option<VarTargetSel> = None;

        var_attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let lit: LitStr = meta.value()?.parse()?;
                name = Some(lit.value());
                return Ok(());
            }
            if meta.path.is_ident("unit") {
                let lit: LitStr = meta.value()?.parse()?;
                unit = Some(lit.value());
                return Ok(());
            }
            if meta.path.is_ident("kind") {
                // Allow: kind = "A" | "AVar" | "L" | "LVar" OR kind = A | L
                let expr: Expr = meta.value()?.parse()?;
                let (value, span) = match expr {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s),
                        ..
                    }) => (s.value(), s.span()),
                    Expr::Path(ExprPath { path, .. }) => {
                        let seg = path
                            .segments
                            .last()
                            .ok_or_else(|| syn::Error::new(path.span(), "invalid kind value"))?;
                        (seg.ident.to_string(), seg.ident.span())
                    }
                    other => {
                        return Err(syn::Error::new(
                            other.span(),
                            "kind must be a string literal (\"A\"/\"L\") or an identifier (A/L)",
                        ));
                    }
                };

                kind = Some(parse_kind_str(&value, span)?);
                return Ok(());
            }
            if meta.path.is_ident("index") {
                let lit: LitInt = meta.value()?.parse()?;
                index = Some(lit.base10_parse::<u32>()?);
                return Ok(());
            }
            if meta.path.is_ident("target") {
                // target = "USER_AIRCRAFT" | "USER_AVATAR" | "USER_CURRENT" OR target = USER_CURRENT
                let expr: Expr = meta.value()?.parse()?;
                let (value, span) = match expr {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s),
                        ..
                    }) => (s.value(), s.span()),
                    Expr::Path(ExprPath { path, .. }) => {
                        let seg = path
                            .segments
                            .last()
                            .ok_or_else(|| syn::Error::new(path.span(), "invalid target value"))?;
                        (seg.ident.to_string(), seg.ident.span())
                    }
                    other => {
                        return Err(syn::Error::new(
                            other.span(),
                            "target must be a string literal (\"USER_CURRENT\") or an identifier (USER_CURRENT)",
                        ));
                    }
                };

                target = Some(parse_target_str(&value, span)?);
                return Ok(());
            }

            Err(meta.error("unsupported #[var(...)] key"))
        })?;

        let name = name.ok_or_else(|| syn::Error::new(var_attr.span(), "#[var] requires name"))?;
        let unit = unit.unwrap_or_else(|| "Number".to_string());
        let kind = kind.or_else(|| infer_kind_from_name(&name));
        let Some(kind) = kind else {
            return Err(syn::Error::new(
                var_attr.span(),
                r#"#[var] requires kind ("A"/"L") or a name prefixed with "A:" or "L:""#,
            ));
        };

        if index.is_some() && kind != VarKindSel::A {
            return Err(syn::Error::new(
                var_attr.span(),
                "#[var(index = ...)] is only supported for kind = A (AVar)",
            ));
        }

        specs.push(FieldSpec {
            ident,
            name,
            unit,
            kind,
            index,
            target,
        });
    }

    if specs.is_empty() {
        return Err(syn::Error::new(
            struct_ident.span(),
            "VarStruct requires at least one #[var(...)] field",
        ));
    }

    let helpers = specs.iter().map(|spec| {
        let field_ident = &spec.ident;

        let helper_fn_ident =
            format_ident!("__msfs_varstruct_get_var_{}_{}", struct_ident, field_ident);
        let cell_ident = format_ident!("__MSFS_VARSTRUCT_CELL_{}_{}", struct_ident, field_ident);

        let name_lit = LitStr::new(&spec.name, field_ident.span());
        let unit_lit = LitStr::new(&spec.unit, field_ident.span());

        let var_ty = match spec.kind {
            VarKindSel::A => quote!(::msfs::vars::a_var::AVar),
            VarKindSel::L => quote!(::msfs::vars::l_var::LVar),
        };

        quote! {
            #[inline]
            fn #helper_fn_ident() -> ::msfs::vars::VarResult<#var_ty> {
                static #cell_ident: ::std::sync::OnceLock<::msfs::vars::VarResult<#var_ty>> =
                    ::std::sync::OnceLock::new();

                match #cell_ident.get_or_init(|| #var_ty::new(#name_lit, #unit_lit)) {
                    Ok(v) => Ok(*v),
                    Err(e) => Err(e.clone()),
                }
            }
        }
    });

    let get_inits = specs.iter().map(|spec| {
        let field_ident = &spec.ident;
        let helper_fn_ident =
            format_ident!("__msfs_varstruct_get_var_{}_{}", struct_ident, field_ident);

        let target_expr = spec.target.map(target_to_tokens);
        let index_expr = spec.index;

        match (index_expr, target_expr) {
            (Some(index), Some(target)) => {
                quote!(#field_ident: #helper_fn_ident()?.get_indexed_target(#index, #target)?)
            }
            (Some(index), None) => quote!(#field_ident: #helper_fn_ident()?.get_indexed(#index)?),
            (None, Some(target)) => quote!(#field_ident: #helper_fn_ident()?.get_target(#target)?),
            (None, None) => quote!(#field_ident: #helper_fn_ident()?.get()?),
        }
    });

    let set_stmts = specs.iter().map(|spec| {
        let field_ident = &spec.ident;
        let helper_fn_ident =
            format_ident!("__msfs_varstruct_get_var_{}_{}", struct_ident, field_ident);

        let target_expr = spec.target.map(target_to_tokens);
        let index_expr = spec.index;

        match (index_expr, target_expr) {
            (Some(index), Some(target)) => {
                quote!(#helper_fn_ident()?.set_indexed_target(#index, #target, self.#field_ident)?;)
            }
            (Some(index), None) => {
                quote!(#helper_fn_ident()?.set_indexed(#index, self.#field_ident)?;)
            }
            (None, Some(target)) => {
                quote!(#helper_fn_ident()?.set_target(#target, self.#field_ident)?;)
            }
            (None, None) => quote!(#helper_fn_ident()?.set(self.#field_ident)?;),
        }
    });

    let expanded = quote! {
        impl #struct_ident {
            #(#helpers)*

            #[inline]
            pub fn get() -> ::msfs::vars::VarResult<Self> {
                Ok(Self { #(#get_inits,)* })
            }

            #[inline]
            pub fn set(&self) -> ::msfs::vars::VarResult<()> {
                #(#set_stmts)*
                Ok(())
            }
        }
    };

    Ok(expanded.into())
}

fn is_f64_type(ty: &syn::Type) -> bool {
    let syn::Type::Path(p) = ty else {
        return false;
    };
    if p.qself.is_some() {
        return false;
    }
    let Some(seg) = p.path.segments.last() else {
        return false;
    };
    seg.ident == "f64"
}

fn parse_kind_str(s: &str, span: proc_macro2::Span) -> syn::Result<VarKindSel> {
    match s.trim() {
        "A" | "AVar" | "a" | "avar" => Ok(VarKindSel::A),
        "L" | "LVar" | "l" | "lvar" => Ok(VarKindSel::L),
        other => Err(syn::Error::new(
            span,
            format!("unknown var kind: {other} (expected A/AVar or L/LVar)"),
        )),
    }
}

fn infer_kind_from_name(name: &str) -> Option<VarKindSel> {
    let upper = name.trim_start().to_ascii_uppercase();
    if upper.starts_with("A:") {
        Some(VarKindSel::A)
    } else if upper.starts_with("L:") {
        Some(VarKindSel::L)
    } else {
        None
    }
}

fn parse_target_str(s: &str, span: proc_macro2::Span) -> syn::Result<VarTargetSel> {
    let norm = s.trim().to_ascii_uppercase();
    match norm.as_str() {
        "USER_AIRCRAFT" | "FS_OBJECT_ID_USER_AIRCRAFT" => Ok(VarTargetSel::UserAircraft),
        "USER_AVATAR" | "FS_OBJECT_ID_USER_AVATAR" => Ok(VarTargetSel::UserAvatar),
        "USER_CURRENT" | "FS_OBJECT_ID_USER_CURRENT" => Ok(VarTargetSel::UserCurrent),
        other => Err(syn::Error::new(
            span,
            format!("unknown target: {other} (expected USER_AIRCRAFT/USER_AVATAR/USER_CURRENT)"),
        )),
    }
}

fn target_to_tokens(t: VarTargetSel) -> proc_macro2::TokenStream {
    match t {
        VarTargetSel::UserAircraft => quote!(::msfs::sys::FS_OBJECT_ID_USER_AIRCRAFT),
        VarTargetSel::UserAvatar => quote!(::msfs::sys::FS_OBJECT_ID_USER_AVATAR),
        VarTargetSel::UserCurrent => quote!(::msfs::sys::FS_OBJECT_ID_USER_CURRENT),
    }
}
