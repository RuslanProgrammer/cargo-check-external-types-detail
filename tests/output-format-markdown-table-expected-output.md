| External crate | External type | Exposure kind | API path | Local struct chain | Role | Source |
| --- | --- | --- | --- | --- | --- | --- |
| external_lib | external_lib::AssociatedGenericTrait | unapproved_external | test_crate::SomeTraitWithExternalDefaultTypes::OtherThing |  | trait bound of | test-crate/src/lib.rs:135:5 |
| external_lib | external_lib::AssociatedGenericTrait | unapproved_external | test_crate::fn_with_external_trait_bounds |  | trait bound of | test-crate/src/lib.rs:124:1 |
| external_lib | external_lib::ReprCType | unapproved_external | test_crate::test_union::SimpleUnion::repr_c |  | return value of | test-crate/src/test_union.rs:15:5 |
| external_lib | external_lib::ReprCType | unapproved_external | test_crate::test_union::SimpleUnion::repr_c |  | struct field of | test-crate/src/test_union.rs:10:5 |
| external_lib | external_lib::SimpleGenericTrait | unapproved_external | test_crate::test_structs::ImplsGenericTrait |  | implemented trait of | test-crate/src/test_structs.rs:27:1 |
| external_lib | external_lib::SimpleNewType | unapproved_external | test_crate::AssocConstStruct::OTHER_CONST |  | struct field of | test-crate/src/lib.rs:157:5 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::DynExternalReferencingTypeAlias |  | dyn trait of | test-crate/src/lib.rs:121:1 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::EnumWithExternals::StructEnum::simple_trait |  | dyn trait of | test-crate/src/lib.rs:91:9 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::EnumWithExternals::TupleEnum::1 |  | dyn trait of | test-crate/src/lib.rs:88:27 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::EnumWithExternals::another_thing |  | trait bound of | test-crate/src/lib.rs:103:5 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::SomeTraitWithExternalDefaultTypes::Thing |  | trait bound of | test-crate/src/lib.rs:134:5 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::SomeTraitWithGenericAssociatedType::MyGAT |  | trait bound of | test-crate/src/lib.rs:145:5 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::SomeTraitWithGenericAssociatedType::some_fn |  | trait bound of | test-crate/src/lib.rs:149:5 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::external_in_fn_input |  | argument named `_two` of | test-crate/src/lib.rs:37:1 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::external_in_fn_input |  | trait bound of | test-crate/src/lib.rs:37:1 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::external_opaque_type_in_output |  | return value of | test-crate/src/lib.rs:46:1 |
| external_lib | external_lib::SimpleTrait | unapproved_external | test_crate::test_union::GenericUnion |  | trait bound of | test-crate/src/test_union.rs:21:1 |
| external_lib | external_lib::SomeOtherStruct | unapproved_external | test_crate::SomeTraitWithExternalDefaultTypes::OtherThing |  | generic default binding of | test-crate/src/lib.rs:135:5 |
| external_lib | external_lib::SomeOtherStruct | unapproved_external | test_crate::StructWithExternalFields::new |  | generic arg of | test-crate/src/lib.rs:71:5 |
| external_lib | external_lib::SomeOtherStruct | unapproved_external | test_crate::fn_with_external_trait_bounds |  | generic arg of | test-crate/src/lib.rs:124:1 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::StructWithExternalFields::field | test_crate::StructWithExternalFields | struct field of | test-crate/src/lib.rs:66:5 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::StructWithExternalFields::optional_field | test_crate::StructWithExternalFields | generic arg of | test-crate/src/lib.rs:67:5 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::test_structs::DoublyNestedStruct::middle | test_crate::test_structs::DoublyNestedStruct → test_crate::test_structs::StructContainingPlainStruct → test_crate::test_structs::PlainStructWithExternalType | struct field of | test-crate/src/test_structs.rs:42:5 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::test_structs::PlainStructWithExternalType::external | test_crate::test_structs::PlainStructWithExternalType | struct field of | test-crate/src/test_structs.rs:14:5 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::test_structs::StructContainingPlainStruct::inner | test_crate::test_structs::StructContainingPlainStruct → test_crate::test_structs::PlainStructWithExternalType | struct field of | test-crate/src/test_structs.rs:34:5 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::test_structs::StructContainingTupleStruct::inner | test_crate::test_structs::StructContainingTupleStruct → test_crate::test_structs::TupleStructWithExternalType | struct field of | test-crate/src/test_structs.rs:38:5 |
| external_lib | external_lib::SomeStruct | local_struct_chain | test_crate::test_structs::TupleStructWithExternalType::0 | test_crate::test_structs::TupleStructWithExternalType | struct field of | test-crate/src/test_structs.rs:8:40 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::EnumWithExternals |  | generic default binding of | test-crate/src/lib.rs:83:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::EnumWithExternals::StructEnum::some_struct |  | struct field of | test-crate/src/lib.rs:90:9 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::EnumWithExternals::TupleEnum::0 |  | struct field of | test-crate/src/lib.rs:88:15 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::ExternalReferencingRawPtr |  | type alias of | test-crate/src/lib.rs:122:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::ExternalReferencingTypeAlias |  | type alias of | test-crate/src/lib.rs:119:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::OptionalExternalReferencingTypeAlias |  | generic arg of | test-crate/src/lib.rs:120:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::SOME_CONST |  | constant | test-crate/src/lib.rs:109:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::SOME_STRUCT |  | static value | test-crate/src/lib.rs:108:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::SomeTraitWithExternalDefaultTypes::OtherThing |  | generic default binding of | test-crate/src/lib.rs:135:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::StructWithExternalFields::field |  | struct field of | test-crate/src/lib.rs:66:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::StructWithExternalFields::new |  | generic arg of | test-crate/src/lib.rs:71:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::StructWithExternalFields::optional_field |  | generic arg of | test-crate/src/lib.rs:67:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::TraitReferencingExternals::optional_otherthing |  | generic arg of | test-crate/src/lib.rs:80:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::TraitReferencingExternals::optional_something |  | generic arg of | test-crate/src/lib.rs:78:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::TraitReferencingExternals::otherthing |  | return value of | test-crate/src/lib.rs:79:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::TraitReferencingExternals::something |  | argument named `a` of | test-crate/src/lib.rs:77:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::external_in_fn_input |  | argument named `_one` of | test-crate/src/lib.rs:37:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::external_in_fn_output |  | return value of | test-crate/src/lib.rs:42:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::external_in_fn_output_generic |  | generic arg of | test-crate/src/lib.rs:53:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::fn_with_external_trait_bounds |  | generic arg of | test-crate/src/lib.rs:124:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::some_pub_mod::OPTIONAL_CONST |  | generic arg of | test-crate/src/lib.rs:115:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::some_pub_mod::OPTIONAL_STRUCT |  | generic arg of | test-crate/src/lib.rs:114:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::something::something |  | argument named `_one` of | test-crate/src/lib.rs:61:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::test_assoc_type::PublicStructImplsPublicTraitWithAssocType::Something |  | generic arg of | test-crate/src/test_assoc_type.rs:55:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::test_assoc_type::PublicStructImplsTraitWithExtAssocType::Error |  | associated type | test-crate/src/test_assoc_type.rs:12:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::test_structs::ImplsGenericTrait |  | generic arg of | test-crate/src/test_structs.rs:27:1 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::test_structs::PlainStructWithExternalType::external |  | struct field of | test-crate/src/test_structs.rs:14:5 |
| external_lib | external_lib::SomeStruct | unapproved_external | test_crate::test_structs::TupleStructWithExternalType::0 |  | struct field of | test-crate/src/test_structs.rs:8:40 |
