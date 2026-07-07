#pragma once

namespace graphene { namespace protocol {

enum asset_issuer_permission_flags {
   charge_market_fee    = 0x01, ///< market trades in this asset may be charged
   white_list           = 0x02, ///< accounts must be whitelisted in order to hold or transact this asset
   override_authority   = 0x04, ///< issuer may transfer asset back to himself
   transfer_restricted  = 0x08, ///< require the issuer to be one party to every transfer
   disable_force_settle = 0x10, ///< disable force settling
   global_settle        = 0x20  ///< allow the bitasset owner to force a global settlement
};

} } // graphene::protocol

GRAPHENE_DEFINE_IDS(protocol, protocol_ids, /*protocol objects are not prefixed*/,
                    /* 1.0.x  */ (null) // no data
                    /* 1.1.x  */ (base) // no data
                    /* 1.2.x  */ (account)
                    /* 1.3.x  */ (asset))

// Note: members deliberately listed in a different order than the enum body,
// matching the real BitShares/Swaplock sources.
FC_REFLECT_ENUM( graphene::protocol::asset_issuer_permission_flags,
   (charge_market_fee)
   (white_list)
   (transfer_restricted)
   (override_authority)
   (disable_force_settle)
   (global_settle)
   )
