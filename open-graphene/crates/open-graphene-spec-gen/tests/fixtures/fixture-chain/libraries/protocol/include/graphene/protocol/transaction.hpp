#pragma once
#include <graphene/protocol/operations.hpp>

namespace graphene { namespace protocol {

   /**
    * @brief groups operations that should be applied atomically
    */
   struct transaction
   {
      /**
       * Least significant 16 bits from the reference block number.
       */
      uint16_t           ref_block_num    = 0;
      /**
       * The first non-block-number 32-bits of the reference block ID.
       */
      uint32_t           ref_block_prefix = 0;
      /**
       * This field specifies the absolute expiration for this transaction.
       */
      fc::time_point_sec expiration;

      vector<operation>  operations;
      extensions_type    extensions;

      /// Calculate the digest for a transaction
      digest_type         digest()const;
      void                validate() const;
   };

   /**
    * @brief adds a signature to a transaction
    */
   struct signed_transaction : public transaction
   {
      signed_transaction( const transaction& trx = transaction() )
         : transaction(trx){}

      /** signs and appends to signatures */
      const signature_type& sign( const private_key_type& key, const chain_id_type& chain_id );

      vector<signature_type> signatures;

      /// Removes all operations and signatures
      void clear() { operations.clear(); signatures.clear(); }
   };

} } // graphene::protocol

FC_REFLECT( graphene::protocol::transaction, (ref_block_num)(ref_block_prefix)(expiration)(operations)(extensions) )
FC_REFLECT_DERIVED( graphene::protocol::signed_transaction, (graphene::protocol::transaction), (signatures) )
