#pragma once
#include <graphene/protocol/transfer.hpp>

namespace graphene { namespace protocol {

   /**
    * @ingroup operations
    * Defines the set of valid operations as a discriminated union type.
    */
   using operation = fc::static_variant<
            /*  0 */ transfer_operation,
            /*  1 */ override_transfer_operation
         >;

} } // graphene::protocol
