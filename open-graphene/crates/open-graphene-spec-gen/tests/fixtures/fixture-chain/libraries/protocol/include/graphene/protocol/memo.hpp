#pragma once
#include <graphene/protocol/types.hpp>

namespace graphene { namespace protocol {

   /**
    *  @brief defines the keys used to derive the shared secret
    */
   struct memo_data
   {
      public_key_type from;
      public_key_type to;
      /**
       * 64 bit nonce format:
       * [  8 bits | 56 bits   ]
       * [ entropy | timestamp ]
       */
      uint64_t nonce = 0;
      /**
       * This field contains the AES encrypted packed @ref memo_message
       */
      vector<char> message;

      void        set_message(const fc::ecc::private_key& priv,
                              const fc::ecc::public_key& pub, const string& msg, uint64_t custom_nonce = 0);

      std::string get_message(const fc::ecc::private_key& priv,
                              const fc::ecc::public_key& pub)const;
   };

} } // namespace graphene::protocol

FC_REFLECT( graphene::protocol::memo_data, (from)(to)(nonce)(message) )
