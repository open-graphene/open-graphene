#pragma once
#include <graphene/protocol/types.hpp>

namespace graphene { namespace protocol {

   struct asset
   {
      asset( share_type a = 0, asset_id_type id = asset_id_type() )
      :amount(a),asset_id(id){}

      share_type    amount;
      asset_id_type asset_id;

      asset& operator += ( const asset& o )
      {
         amount += o.amount;
         return *this;
      }

      friend bool operator < ( const asset& a, const asset& b )
      {
         return std::tie(a.asset_id, a.amount) < std::tie(b.asset_id, b.amount);
      }
   };

} } // graphene::protocol

FC_REFLECT( graphene::protocol::asset, (amount)(asset_id) )
