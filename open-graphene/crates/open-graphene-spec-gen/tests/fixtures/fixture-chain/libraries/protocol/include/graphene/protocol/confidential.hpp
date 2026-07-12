#pragma once
#include <graphene/protocol/types.hpp>
#include <graphene/protocol/asset.hpp>

namespace graphene { namespace protocol {

/**
 *  @ingroup stealth
 */
struct stealth_confirmation
{
   struct memo_data
   {
      optional<public_key_type> from;
      asset                     amount;
      fc::sha256                blinding_factor;
      fc::ecc::commitment_type  commitment;
      uint32_t                  check = 0;
   };

   public_key_type           one_time_key;
   optional<public_key_type> to;
   vector<char>              encrypted_memo_data;
};

} } // graphene::protocol

FC_REFLECT( graphene::protocol::stealth_confirmation::memo_data,
            (from)(amount)(blinding_factor)(commitment)(check) )
FC_REFLECT( graphene::protocol::stealth_confirmation,
            (one_time_key)(to)(encrypted_memo_data) )
