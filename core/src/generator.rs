use syn;
use proc_macro::{TokenStream, Diagnostic};
use proc_macro2::TokenStream as TokenStream2;

use ext::{CodegenFieldsExt, PathExt};
use spanned::Spanned;

use field::Field;
use variant::Variant;
use support::{GenericSupport, DataSupport};

pub type Result<T> = ::std::result::Result<T, Diagnostic>;

pub type EnumValidatorFn = fn(&DeriveGenerator, &::syn::DataEnum) -> Result<()>;
pub type StructValidatorFn = fn(&DeriveGenerator, &::syn::DataStruct) -> Result<()>;

pub type TypeGenericMapFn = fn(&DeriveGenerator, &syn::Ident, &syn::TypeParam) -> TokenStream2;

pub type FunctionFn = fn(&DeriveGenerator, TokenStream2) -> TokenStream2;
pub type EnumMapFn = fn(&DeriveGenerator, &syn::DataEnum) -> TokenStream2;
pub type StructMapFn = fn(&DeriveGenerator, &syn::DataStruct) -> TokenStream2;
pub type VariantMapFn = fn(&DeriveGenerator, Variant) -> TokenStream2;
pub type FieldMapFn = fn(&DeriveGenerator, Field) -> TokenStream2;

#[derive(Clone)]
pub struct DeriveGenerator {
    pub input: syn::DeriveInput,
    pub trait_path: syn::Path,
    crate generic_support: GenericSupport,
    crate data_support: DataSupport,
    crate enum_validator: EnumValidatorFn,
    crate struct_validator: StructValidatorFn,
    crate type_generic_mapper: Option<TypeGenericMapFn>,
    crate functions: Vec<FunctionFn>,
    crate enum_mappers: Vec<EnumMapFn>,
    crate struct_mappers: Vec<StructMapFn>,
    crate variant_mappers: Vec<VariantMapFn>,
    crate field_mappers: Vec<FieldMapFn>,
}

pub fn default_enum_mapper(gen: &DeriveGenerator, data: &syn::DataEnum) -> TokenStream2 {
    let variant = data.variants.iter().map(|v| &v.ident);
    let fields = data.variants.iter().map(|v| v.fields.id_match_tokens());
    let enum_name = ::std::iter::repeat(&gen.input.ident);
    let expression = data.variants.iter().map(|variant| {
        let variant = Variant { parent: gen.input.ident.clone(), variant };
        gen.variant_mapper()(gen, variant)
    });

    quote! {
        match self {
            #(#enum_name::#variant #fields => #expression),*
        }
    }
}

pub fn null_enum_mapper(gen: &DeriveGenerator, data: &syn::DataEnum) -> TokenStream2 {
    let tokens = data.variants.iter().map(|variant| {
        let variant = Variant { parent: gen.input.ident.clone(), variant };
        gen.variant_mapper()(gen, variant)
    });

    quote!(#(#tokens)*)
}

pub fn default_struct_mapper(gen: &DeriveGenerator, data: &syn::DataStruct) -> TokenStream2 {
    let field = data.fields.iter().enumerate().map(|(index, field)| {
        let wrapped_field = Field { matched: false, index, field };
        gen.field_mapper()(gen, wrapped_field)
    });

    quote!(#(#field)*)
}

pub fn default_variant_mapper(gen: &DeriveGenerator, data: Variant) -> TokenStream2 {
    let field = data.fields.iter().enumerate().map(|(index, field)| {
        let wrapped_field = Field { matched: true, index, field };
        gen.field_mapper()(gen, wrapped_field)
    });

    quote!({ #(#field)* })
}

pub fn default_field_mapper(_gen: &DeriveGenerator, _data: Field) -> TokenStream2 {
    panic!("must override field or something above")
}

impl DeriveGenerator {
    pub fn build_for(input: TokenStream, trait_path: &str) -> DeriveGenerator {
        let trait_path_ts = trait_path.parse().expect("invalid tokens for path");
        let trait_path = syn::parse(trait_path_ts).expect("invalid trait path");
        let input = syn::parse(input).expect("invalid derive input");

        DeriveGenerator {
            input, trait_path,
            generic_support: GenericSupport::None,
            data_support: DataSupport::None,
            type_generic_mapper: None,
            enum_validator: |_, _| Ok(()),
            struct_validator: |_, _| Ok(()),
            functions: vec![],
            enum_mappers: vec![],
            struct_mappers: vec![],
            variant_mappers: vec![],
            field_mappers: vec![],
        }
    }

    pub fn generic_support(&mut self, support: GenericSupport) -> &mut Self {
        self.generic_support = support;
        self
    }

    pub fn data_support(&mut self, support: DataSupport) -> &mut Self {
        self.data_support = support;
        self
    }

    pub fn validate_enum(&mut self, f: EnumValidatorFn) -> &mut Self {
        self.enum_validator = f;
        self
    }

    pub fn validate_struct(&mut self, f: StructValidatorFn) -> &mut Self {
        self.struct_validator = f;
        self
    }

    pub fn push_default_mappers(&mut self) {
        self.enum_mappers.push(default_enum_mapper);
        self.struct_mappers.push(default_struct_mapper);
        self.variant_mappers.push(default_variant_mapper);
        self.field_mappers.push(default_field_mapper);
    }

    pub fn map_type_generic(&mut self, f: TypeGenericMapFn) -> &mut Self {
        self.type_generic_mapper = Some(f);
        self
    }

    pub fn function(&mut self, f: FunctionFn) -> &mut Self {
        self.functions.push(f);
        self.push_default_mappers();
        self
    }

    pub fn map_variant(&mut self, f: VariantMapFn) -> &mut Self {
        if !self.variant_mappers.is_empty() {
            let last = self.variant_mappers.len() - 1;
            self.variant_mappers[last] = f;
        }

        self
    }

    pub fn map_enum(&mut self, f: EnumMapFn) -> &mut Self {
        if !self.enum_mappers.is_empty() {
            let last = self.enum_mappers.len() - 1;
            self.enum_mappers[last] = f;
        }

        self
    }

    pub fn map_field(&mut self, f: FieldMapFn) -> &mut Self {
        if !self.field_mappers.is_empty() {
            let last = self.field_mappers.len() - 1;
            self.field_mappers[last] = f;
        }

        self
    }

    pub fn variant_mapper(&self) -> &VariantMapFn {
        assert!(!self.variant_mappers.is_empty());
        let last = self.variant_mappers.len() - 1;
        &self.variant_mappers[last]
    }

    pub fn field_mapper(&self) -> &FieldMapFn {
        assert!(!self.field_mappers.is_empty());
        let last = self.field_mappers.len() - 1;
        &self.field_mappers[last]
    }

    fn _to_tokens(&mut self) -> Result<TokenStream> {
        use syn::*;

        // Step 1: Run all validators.
        // Step 1a: First, check for data support.
        match self.input.data {
            Data::Struct(..) if !self.data_support.contains(DataSupport::Struct) => {
                return Err(self.input.span().error("unsupported data kind: struct"));
            }
            Data::Enum(..) if !self.data_support.contains(DataSupport::Enum) => {
                return Err(self.input.span().error("unsupported data kind: enum"));
            }
            Data::Union(..) if !self.data_support.contains(DataSupport::Union) => {
                return Err(self.input.span().error("unsupported data kind: union"));
            }
            _ => { /* we're okay! */ }
        }

        // Step 1b: Second, check for generics support.
        for generic in &self.input.generics.params {
            use syn::GenericParam::*;
            match generic {
                Type(..) if !self.generic_support.contains(GenericSupport::Type) => {
                    return Err(generic.span().error("type generics are not supported"));
                }
                Lifetime(..) if !self.generic_support.contains(GenericSupport::Lifetime) => {
                    return Err(generic.span().error("lifetime generics are not supported"));
                }
                Const(..) if !self.generic_support.contains(GenericSupport::Const) => {
                    return Err(generic.span().error("const generics are not supported"));
                }
                _ => { /* we're okay! */ }
            }
        }

        // Step 1c: Third, run the custom validators.
        match self.input.data {
            Data::Struct(ref data) => (self.struct_validator)(self, data)?,
            Data::Union(ref _data) => unimplemented!("can't custom validate unions yet"),
            Data::Enum(ref data) => (self.enum_validator)(self, data)?,
        }

        // Step 2: Generate the code!
        // Step 2a: Generate the code for each function.
        let mut function_code = vec![];
        for i in 0..self.functions.len() {
            let function = self.functions[i];
            let inner = match self.input.data {
                Data::Struct(ref data) => self.struct_mappers[i](self, data),
                Data::Union(ref _data) => unimplemented!("can't gen unions yet"),
                Data::Enum(ref data) => self.enum_mappers[i](self, data),
            };

            function_code.push(function(self, inner));
        }

        // Step 2b: Create a couple of generics to mutate with user's input.
        let mut generics = self.input.generics.clone();

        // Step 2c: Add additional where bounds if the user asked for it.
        if let Some(type_mapper) = self.type_generic_mapper {
            for ty in self.input.generics.type_params() {
                let clause = syn::parse2(type_mapper(self, &ty.ident, ty)).expect("yo");
                generics.make_where_clause().predicates.push(clause);
            }
        }

        // Step 2d: Add any generics the user supplied.
        // TODO: Allow the user to say we should replace generics from the
        // struct with generics from the trait.
        let mut generics_for_impl_generics = generics.clone();
        if let Some(trait_generics) = self.trait_path.generics() {
            for generic in trait_generics {
                if let syn::GenericArgument::Lifetime(lifetime) = generic {
                    let param = LifetimeDef::new(lifetime.clone());
                    generics_for_impl_generics.params.insert(0, param.into());
                } else {
                    unimplemented!("can only handle lifetime generics in traits")
                }
            }
        }

        // Step 2e: Split the generics, but use the `impl_generics` from above.
        let (impl_gen, _, _) = generics_for_impl_generics.split_for_impl();
        let (_, ty_gen, where_gen) = generics.split_for_impl();

        // FIXME: Check if we can also use id_match_tokens due to match
        // ergonomics. I don't think so, though. If we can't, then ask (in
        // `function`) whether receiver is `&self`, `&mut self` or `self` and
        // bind match accordingly.
        // TODO: Port FromForm to this.
        // TODO: Port Responder to this.

        // Step 2b: Generate the complete implementation.
        let target = &self.input.ident;
        let trait_name = &self.trait_path;
        Ok(quote! {
            impl #impl_gen #trait_name for #target #ty_gen #where_gen {
                #(#function_code)*
            }
        }.into())
    }

    pub fn to_tokens(&mut self) -> TokenStream {
        // FIXME: Emit something like: Trait: msg.
        self._to_tokens()
            .unwrap_or_else(|diag| {
                if let Some(last) = self.trait_path.segments.last() {
                    use proc_macro::Span;
                    use proc_macro::Level::*;

                    let id = &last.value().ident;
                    let msg = match diag.level() {
                        Error => format!("error occurred while deriving `{}`", id),
                        Warning => format!("warning issued by `{}` derive", id),
                        Note => format!("note issued by `{}` derive", id),
                        Help => format!("help provided by `{}` derive", id),
                        _ => format!("while deriving `{}`", id)
                    };

                    diag.span_note(Span::call_site(), msg).emit();
                }

                TokenStream::new().into()
            })
    }
}
