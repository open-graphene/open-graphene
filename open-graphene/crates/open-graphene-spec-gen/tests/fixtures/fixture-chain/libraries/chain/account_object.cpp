#include <graphene/chain/account_object.hpp>

namespace graphene { namespace chain {

// implementation detail omitted

} } // graphene::chain

FC_REFLECT_DERIVED_NO_TYPENAME( graphene::chain::account_object, (graphene::db::object),
                    (membership_expiration_date)(name)(options) )
