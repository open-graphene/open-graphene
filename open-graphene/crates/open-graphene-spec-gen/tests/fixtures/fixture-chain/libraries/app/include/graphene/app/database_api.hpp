#pragma once
#include <graphene/chain/account_object.hpp>
#include <graphene/protocol/types.hpp>

namespace graphene { namespace app {

using namespace graphene::chain;

class database_api_impl;

/**
 * @brief The database_api class implements the RPC API for the chain database.
 */
class database_api
{
   public:
      /**
       * @brief Get the chain ID
       * @return The chain ID identifying blockchain network
       */
      chain_id_type get_chain_id()const;

      /**
       * @brief Get a list of accounts by names or IDs
       * @param account_names_or_ids names or stringified IDs of the accounts to retrieve
       * @param subscribe @a true to subscribe to the queried account objects
       * @return The accounts corresponding to the provided names or IDs
       */
      vector<optional<account_object>> get_accounts( const vector<std::string>& account_names_or_ids,
                                                     optional<bool> subscribe = optional<bool>() )const;
};

} } // graphene::app

FC_API(graphene::app::database_api,
   (get_chain_id)
   (get_accounts)
)
