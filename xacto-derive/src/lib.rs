use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DataEnum, DeriveInput, Error, Ident, Type, TypePath, parse_macro_input, spanned::Spanned,
};

struct MessageVariant {
    ident: Ident,
    variant: Ident,
    request_fields: Vec<Type>,
    response_type: Option<Type>,
}

impl MessageVariant {
    fn get_request_variant(&self) -> proc_macro2::TokenStream {
        let ident = &self.variant;
        if self.request_fields.is_empty() {
            quote! { #ident }
        } else {
            let fields = &self.request_fields;
            quote! { #ident ( #(#fields),* ) }
        }
    }

    fn get_response_variant(&self) -> Option<proc_macro2::TokenStream> {
        if let Some(response_type) = &self.response_type {
            let ident = &self.variant;
            Some(quote! { #ident ( #response_type ) })
        } else {
            None
        }
    }

    fn get_original_arm(&self) -> proc_macro2::TokenStream {
        let o_ident = &self.ident;
        let v_ident = &self.variant;
        let mut fields = vec![];
        for i in 0..self.request_fields.len() {
            let field_ident = format_ident!("a{}", i);
            fields.push(quote! { #field_ident });
        }
        if self.response_type.is_some() {
            let reply = quote! { reply };
            fields.push(reply);
        }
        if fields.is_empty() {
            quote! { #o_ident::#v_ident }
        } else {
            quote! { #o_ident::#v_ident ( #(#fields),* ) }
        }
    }

    fn get_request_arm(&self) -> proc_macro2::TokenStream {
        let v_ident = &self.variant;
        let mut fields = vec![];
        for i in 0..self.request_fields.len() {
            let field_ident = format_ident!("a{}", i);
            fields.push(quote! { #field_ident });
        }
        if fields.is_empty() {
            quote! { Self::Request::#v_ident }
        } else {
            quote! { Self::Request::#v_ident ( #(#fields),* ) }
        }
    }

    fn get_response_arm(&self) -> proc_macro2::TokenStream {
        let v_ident = &self.variant;
        quote! { Self::Response::#v_ident ( response ) }
    }
}

#[proc_macro_derive(RpcMessage)]
pub fn rpc_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse_rpc_message(input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn parse_rpc_message(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Enum(DataEnum { variants, .. }) = &input.data else {
        return Err(Error::new(
            input.span(),
            "RpcMessage can only be derived for enums",
        ));
    };

    let mut new_variants = vec![];

    for v in variants {
        match &v.fields {
            syn::Fields::Named(fields) => {
                return Err(Error::new(fields.span(), "Named fields are not supported"));
            }
            syn::Fields::Unnamed(fields) => {
                let mut mv = MessageVariant {
                    ident: input.ident.clone(),
                    variant: v.ident.clone(),
                    request_fields: vec![],
                    response_type: None,
                };

                for field in &fields.unnamed {
                    if is_reply_type(&field.ty) {
                        if mv.response_type.is_some() {
                            return Err(Error::new(
                                field.span(),
                                "Only one reply type is allowed per variant",
                            ));
                        }
                        let inner_ty = get_inner_reply_type(&field.ty)?;
                        mv.response_type = Some(inner_ty);
                    } else {
                        mv.request_fields.push(field.ty.clone());
                    }
                }

                new_variants.push(mv);
            }
            syn::Fields::Unit => {
                new_variants.push(MessageVariant {
                    ident: input.ident.clone(),
                    variant: v.ident.clone(),
                    request_fields: vec![],
                    response_type: None,
                });
            }
        }
    }

    let ident = input.ident.clone();
    let request_ident = format_ident!("{}Request", ident);
    let response_ident = format_ident!("{}Response", ident);

    let request_variants = new_variants
        .iter()
        .map(|mv| mv.get_request_variant())
        .collect::<Vec<_>>();

    let response_variants = new_variants
        .iter()
        .filter_map(|mv| mv.get_response_variant())
        .collect::<Vec<_>>();

    let request_enum = quote! {
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        pub enum #request_ident {
            #(#request_variants),*
        }
    };

    let response_enum = if !response_variants.is_empty() {
        Some(quote! {
            #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
            pub enum #response_ident {
                #(#response_variants),*
            }
        })
    } else {
        None
    };

    let mut into_request_arms = vec![];
    let mut proxy_request_arms = vec![];
    let mut proxy_response_arms = vec![];

    for mv in &new_variants {
        let original_arm = mv.get_original_arm();
        let request_arm = mv.get_request_arm();
        let response_arm = mv.get_response_arm();

        if mv.response_type.is_none() {
            into_request_arms.push(quote! {
                #original_arm => ::xacto::RpcEnvelope {
                    id: 0,
                    payload: #request_arm,
                }
            });

            proxy_request_arms.push(quote! {
                #request_arm => {
                    let msg = #original_arm;
                    let (msg, act) = f(msg).ok_or(())?;
                    act.cast(msg).await.map_err(|_| ())?;
                    Ok(None)
                }
            });
        } else {
            into_request_arms.push(quote! {
                #original_arm => {
                    let id = replies.insert_reply(reply);
                    ::xacto::RpcEnvelope {
                        id,
                        payload: #request_arm,
                    }
                }
            });

            proxy_request_arms.push(quote! {
                #request_arm => {
                    let (tx, rx) = ::tokio::sync::oneshot::channel();
                    let reply = Reply::new(tx);
                    let msg = #original_arm;
                    let (msg, act) = f(msg).ok_or(())?;
                    act.cast(msg).await.map_err(|_| ())?;
                    let response = rx.await.unwrap();
                    let env = ::xacto::RpcEnvelope {
                        id: env.id,
                        payload: #response_arm,
                    };
                    Ok(Some(env))
                }
            });

            proxy_response_arms.push(quote! {
                #response_arm => {
                    let reply = replies.get_reply(env.id).ok_or(())?;
                    reply.send(response).map_err(|_| ())?;
                    Ok(())
                }
            });
        }
    }

    let response_assoc_type = if response_variants.is_empty() {
        quote! { () }
    } else {
        quote! { #response_ident}
    };

    let proxy_response_impl = if response_variants.is_empty() {
        quote! {
            unreachable!()
        }
    } else {
        quote! {
            match env.payload {
                #(#proxy_response_arms),*
            }
        }
    };

    let rpc_message_impl = quote! {
        #[::async_trait::async_trait]
        impl ::xacto::RpcMessage for #ident {
            type Request = #request_ident;
            type Response = #response_assoc_type;

            fn into_request(self, replies: &mut ::xacto::ReplyMap) -> ::xacto::RpcEnvelope<Self::Request> {
                match self {
                    #(#into_request_arms),*
                }
            }

            async fn proxy_request<F: Send>(
                env: ::xacto::RpcEnvelope<Self::Request>,
                f: F,
            ) -> Result<Option<::xacto::RpcEnvelope<Self::Response>>, ()>
            where
                F: FnOnce(Self) -> Option<(Self, ::xacto::Act<Self>)>,
                Self: Sized,
            {
                match env.payload {
                    #(#proxy_request_arms),*
                }
            }

            async fn proxy_response(
                env: ::xacto::RpcEnvelope<Self::Response>,
                replies: &mut ::xacto::ReplyMap,
            ) -> Result<(), ()> {
                #proxy_response_impl
            }
        }
    };

    let mut out = request_enum;

    if let Some(response_enum) = response_enum {
        out.extend(response_enum);
    }

    out.extend(rpc_message_impl);

    Ok(out.into())
}

fn is_reply_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { qself: None, path }) => path
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "Reply"),
        _ => false,
    }
}

fn get_inner_reply_type(ty: &Type) -> Result<Type, Error> {
    match ty {
        Type::Path(TypePath { qself: None, path }) => {
            if let Some(segment) = path.segments.last() {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Ok(inner_ty.clone());
                    }
                }
            }
        }
        _ => {}
    };
    Err(Error::new(ty.span(), "Expected Reply<T> type"))
}
