#pragma once
#include <graphene/protocol/types.hpp>

namespace graphene { namespace protocol {

   /**
    *  For future expansion many structures include a single member of type
    *  extensions_type that can be changed when updating a protocol.
    */
   using future_extensions = static_variant<void_t>;

   /**
    *  A flat_set of future_extensions.
    */
   using extensions_type = future_extensions::flat_set_type;

   struct base_operation
   {
      struct fee_params_t {};

      void get_required_authorities( vector<authority>& )const {}
      void validate()const {}
   };

} } // graphene::protocol

FC_REFLECT_TYPENAME( graphene::protocol::future_extensions )
