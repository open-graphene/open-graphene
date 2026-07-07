#pragma once
#include <graphene/protocol/asset.hpp>
#include <graphene/protocol/memo.hpp>

namespace graphene { namespace protocol {

   /**
    * @ingroup operations
    *
    * @brief Transfers an amount of one asset from one account to another
    */
   struct transfer_operation : public base_operation
   {
      struct fee_params_t {
         uint64_t fee       = 20 * GRAPHENE_BLOCKCHAIN_PRECISION;
         uint32_t price_per_kbyte = 10 * GRAPHENE_BLOCKCHAIN_PRECISION; /// only required for large memos.
      };

      asset            fee;
      /// Account to transfer asset from
      account_id_type  from;
      /// Account to transfer asset to
      account_id_type  to;
      /// The amount of asset to transfer from @ref from to @ref to
      asset            amount;

      /// User provided data encrypted to the memo key of the "to" account
      optional<memo_data> memo;
      extensions_type   extensions;

      account_id_type fee_payer()const { return from; }
      void            validate()const;
      share_type      calculate_fee(const fee_params_t& k)const;
   };

} } // graphene::protocol

FC_REFLECT( graphene::protocol::transfer_operation::fee_params_t, (fee)(price_per_kbyte) )
FC_REFLECT( graphene::protocol::transfer_operation, (fee)(from)(to)(amount)(memo)(extensions) )
