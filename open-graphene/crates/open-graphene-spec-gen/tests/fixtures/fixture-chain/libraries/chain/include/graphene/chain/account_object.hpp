#pragma once
#include <graphene/protocol/account.hpp>

namespace graphene { namespace chain {

   /**
    * @brief This class represents an account on the object graph
    * @ingroup object
    *
    * Note: like the real chain sources, this object is reflected in a .cpp
    * translation unit, not in this header.
    */
   class account_object : public graphene::db::abstract_object<account_object, protocol_ids, account_object_type>
   {
      public:
         /// The time at which this account's membership expires.
         time_point_sec membership_expiration_date;

         /// The account's name. This name must be unique among all account names on the graph.
         string name;

         account_options options;
   };

} } // graphene::chain
