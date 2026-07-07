#pragma once
#include <graphene/protocol/types.hpp>

namespace graphene { namespace protocol {

   /**
    * These are the fields which can be updated by the active authority.
    */
   struct account_options
   {
      /// The memo key is the key this account will typically use to encrypt/sign transaction memos.
      public_key_type  memo_key;
      /// The account this account's votes are proxied through.
      account_id_type voting_account;

      /// The number of active witnesses this account votes the blockchain should appoint.
      uint16_t num_witness = 0;
      /// The number of active committee members this account votes the blockchain should appoint.
      uint16_t num_committee = 0;
      /// This is the list of vote IDs this account votes for.
      flat_set<vote_id_type> votes;
      extensions_type        extensions;

      /// Whether this account is voting
      inline bool is_voting() const
      {
         return !votes.empty();
      }

      void validate()const;
   };

} } // graphene::protocol

FC_REFLECT(graphene::protocol::account_options, (memo_key)(voting_account)(num_witness)(num_committee)(votes)(extensions))
