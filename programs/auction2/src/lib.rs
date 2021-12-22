use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},

};

declare_id!("AFuVpFDuKu3SHz7M5FtxmTgmohcqmokqou78h32FiNmF");

#[program]
pub mod auction2 {
    use super::*;
    pub fn new(ctx: Context<Initialize>, _data_bump:u8, mk_cut:u64) -> ProgramResult {
        let data_acc = &mut ctx.accounts.data_acc;
        data_acc.market_place = ctx.accounts.beneficiary.key();
        data_acc.market_place_cut = mk_cut;
        data_acc.pda_rent = ctx.accounts.pda_rent.key();
        
        Ok(())
    }

    pub fn start_auction(ctx: Context<StartAuction>, _data_bump:u8,
    _auction_meta_bump:u8, _base_bid_bump:u8, escrow_bump:u8, auction_valid_till:i64, requestprice:u64) -> ProgramResult {

        let auction_meta = &mut ctx.accounts.auction_meta;
        
        if ctx.accounts.clock.unix_timestamp > auction_valid_till {
            return Err(ProgramError::Custom(0x1));
        }

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.owners_ata.to_account_info(),
                    to: ctx.accounts.escrowed_ata.to_account_info(),
                    // The offer_maker had to sign from the client
                    authority: ctx.accounts.nft_owner.to_account_info(),
                },
            ),
            1,
        )?;
       

        auction_meta.nftowner = ctx.accounts.nft_owner.to_account_info().key();
        auction_meta.nftmint = ctx.accounts.nft_mint.to_account_info().key();
        auction_meta.bid_price = 0;
        auction_meta.bids_placed = 0;
        auction_meta.auction_valid_till = auction_valid_till;
        auction_meta.requestprice = requestprice;
        auction_meta.complete = false;
        auction_meta.escrowed_nft_bump = escrow_bump;
        Ok(())
    }

    pub fn place_bid(ctx: Context<MakeBid>, _data_bump:u8,
    _auction_meta_bump:u8, bid_price:u64) -> ProgramResult {

        let auction_meta = &mut ctx.accounts.auction_meta;
        let bid_acc = &mut ctx.accounts.bid;
        
        if bid_price < auction_meta.requestprice || bid_price < auction_meta.bid_price
        ||  ctx.accounts.clock.unix_timestamp > auction_meta.auction_valid_till || auction_meta.complete {
            
            return Err(ProgramError::Custom(0x2));
        }

        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            ctx.accounts.bid_maker.key,
            bid_acc.to_account_info().key,
            bid_price,
        );

        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[
                ctx.accounts.bid_maker.to_account_info(),
                bid_acc.to_account_info(),
            ],
        )?;

        if auction_meta.bids_placed > 0 {

            **ctx.accounts.previous_bid.to_account_info().try_borrow_mut_lamports()? -= auction_meta.bid_price;
            **ctx.accounts.previous_bidder.to_account_info().try_borrow_mut_lamports()? += auction_meta.bid_price;
        }
        auction_meta.bid_price = bid_price;
        auction_meta.bids_placed = auction_meta.bids_placed + 1;
        auction_meta.activebid = bid_acc.key();

        bid_acc.auction = auction_meta.key();
        bid_acc.bidder = ctx.accounts.bid_maker.to_account_info().key();
        bid_acc.bid_price = bid_price;
        msg!("BIDS PLACED");
        Ok(())
    }

    pub fn redeem(ctx: Context<Redeem>, _data_bump:u8, _auction_meta_bump:u8, _latest_bid_bump:u8, _auction_valid_till:i64) -> ProgramResult {

        let auction_meta = &mut ctx.accounts.auction_meta;
    
        if ctx.accounts.clock.unix_timestamp < auction_meta.auction_valid_till || auction_meta.complete {
            return Err(ProgramError::Custom(0x3));
        }
        //TRANSFER TO THE OWNER
        if (auction_meta.bids_placed == 0) {
            anchor_spl::token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Transfer {
                                from: ctx.accounts.escrowed_ata.to_account_info(),
                                to: ctx.accounts.owners_ata.to_account_info(),
                                authority: ctx.accounts.escrowed_ata.to_account_info(),
                            },
                            &[&[
                                auction_meta.key().as_ref(),
                                &[auction_meta.escrowed_nft_bump],
                            ]],
                        ),
                        // The amount here is just the entire balance of the escrow account.
                    1,
            )?;
    
            anchor_spl::token::close_account(CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::CloseAccount {
                        account: ctx.accounts.escrowed_ata.to_account_info(),
                        destination: ctx.accounts.nft_owner.to_account_info(),
                        authority: ctx.accounts.escrowed_ata.to_account_info(),
                    },
                    &[&[
                        auction_meta.key().as_ref(),
                        &[auction_meta.escrowed_nft_bump],
                    ]],
            ))?;
    
            auction_meta.complete = true;
    
            return Ok(())
            
        }
        
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                            from: ctx.accounts.escrowed_ata.to_account_info(),
                            to: ctx.accounts.latest_bidders_nft_account.to_account_info(),
                            authority: ctx.accounts.escrowed_ata.to_account_info(),
                        },
                        &[&[
                            auction_meta.key().as_ref(),
                            &[auction_meta.escrowed_nft_bump],
                        ]],
                    ),
                    // The amount here is just the entire balance of the escrow account.
                1,
        )?;

    //Finally, close the escrow account and refund the maker (they paid for
    // its rent-exemption).
    anchor_spl::token::close_account(CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::CloseAccount {
                        account: ctx.accounts.escrowed_ata.to_account_info(),
                        destination: ctx.accounts.nft_owner.to_account_info(),
                        authority: ctx.accounts.escrowed_ata.to_account_info(),
                    },
                    &[&[
                        auction_meta.key().as_ref(),
                        &[auction_meta.escrowed_nft_bump],
                    ]],
    ))?;
        let mut taker_amount = auction_meta.bid_price;
            // Multi by 10
        let market_cut = ctx.accounts.data_acc.market_place_cut * taker_amount / 1000;
        let sfb = metaplex_token_metadata::state::Metadata::from_account_info(&ctx.accounts.token_metadata_account)?.data.seller_fee_basis_points;
        let sfb_cut = sfb as u64 * taker_amount / 10000;
        taker_amount = taker_amount - (market_cut + sfb_cut);
        //TRANSFER THE SOL AND THEN THE NFT
    
        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= taker_amount;
        **ctx.accounts.nft_owner.to_account_info().try_borrow_mut_lamports()? += taker_amount;
    
        if *ctx.accounts.market_maker.key != ctx.accounts.data_acc.market_place {
            return Err(ProgramError::Custom(0x4));
        }
        
        //Transfer to Market Maker
        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= market_cut;
        **ctx.accounts.market_maker.to_account_info().try_borrow_mut_lamports()? += market_cut;
    
        if sfb_cut > 0 {    
          
            if let Some(x) = metaplex_token_metadata::state::Metadata::from_account_info(&ctx.accounts.token_metadata_account)?.data.creators {
                let mut y = 0;
    
            for i in x {
                    if y == 0 {
                        if i.address != *ctx.accounts.creator0.key {
                            return Err(ProgramError::Custom(0x5));
                        }
    
                        let temp =  sfb_cut as u64 * i.share as u64 / 100;
                        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= temp;
                        **ctx.accounts.creator0.to_account_info().try_borrow_mut_lamports()? += temp;
                    }
                    else if y == 1 {
                        if i.address != *ctx.accounts.creator1.key {
                            return Err(ProgramError::Custom(0x5));
                        }
                                      
                        let temp =  sfb_cut as u64 * i.share as u64 / 100;
                        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= temp;
                        **ctx.accounts.creator1.to_account_info().try_borrow_mut_lamports()? += temp;
                    }
                    else if y == 2 {
                        if i.address != *ctx.accounts.creator2.key {
                            return Err(ProgramError::Custom(0x5));
                        }
       
                        let temp =  sfb_cut as u64 * i.share as u64 / 100;
                    
                        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= temp;
                        **ctx.accounts.creator2.to_account_info().try_borrow_mut_lamports()? += temp;
                    }
                    else if y == 3 {
                        if i.address != *ctx.accounts.creator3.key {
                            return Err(ProgramError::Custom(0x5));
                        }
    
                        let temp =  sfb_cut as u64 * i.share as u64 / 100;
                     
                        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= temp;
                        **ctx.accounts.creator3.to_account_info().try_borrow_mut_lamports()? += temp;
                    }
                    else if y == 4 {
                        if i.address != *ctx.accounts.creator1.key {
                            return Err(ProgramError::Custom(0x5));
                        }
    
        
                        let temp =  sfb_cut as u64 * i.share as u64 / 100;
                        
                        **ctx.accounts.present_bid_acc.to_account_info().try_borrow_mut_lamports()? -= temp;
                        **ctx.accounts.creator4.to_account_info().try_borrow_mut_lamports()? += temp;
                    }
                    y = y + 1;
    
            }
     
            }
    
        }
    

    
            auction_meta.complete = true;
            Ok(())
    }
    
    
    


}

#[account]
pub struct Data {

    pub market_place: Pubkey,
    
    pub market_place_cut: u64,

    pub pda_rent: Pubkey,

}

#[account]
pub struct NFTAuction {

    pub nftowner: Pubkey,
    pub nftmint: Pubkey,
    pub requestprice: u64,
    pub auction_valid_till: i64,
    pub bid_price: u64,
    pub bids_placed: u64,
    pub activebid: Pubkey,
    pub complete: bool,
    pub escrowed_nft_bump:u8,

}

#[account]
pub struct Bids {

    pub auction: Pubkey,
    pub bidder: Pubkey,
    pub bid_price: u64,
}



#[derive(Accounts)]
#[instruction(data_bump: u8)]
pub struct Initialize<'info> {

    #[account(init, payer=payer, seeds = [b"data".as_ref()], bump = data_bump, space = 8 + 32 + 8 + 32 + 64 + 8)]
    pub data_acc: Account<'info, Data>,

 

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub beneficiary: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    pub pda_rent: AccountInfo<'info>,

}

#[derive(Accounts)]
#[instruction(data_bump: u8, auction_meta_bump:u8, base_bid_bump:u8, escrow_bump:u8, auction_valid_till:i64)]

pub struct StartAuction<'info> {
    #[account( seeds = [b"data".as_ref()], bump = data_bump)]
    pub data_acc: Box<Account<'info, Data>>,

    #[account(init,
    payer = nft_owner,
    seeds = [nft_owner.to_account_info().key.as_ref(),
    nft_mint.to_account_info().key.as_ref(), auction_valid_till.to_be_bytes().as_ref()],
    bump = auction_meta_bump,
    space = 700)]
    pub auction_meta: Box<Account<'info, NFTAuction>>,
    
    #[account(init,
        payer = nft_owner,
        seeds = [nft_owner.to_account_info().key.as_ref(), nft_mint.to_account_info().key.as_ref(), nft_owner.to_account_info().key.as_ref(),
        auction_valid_till.to_be_bytes().as_ref(), (0 as u64).to_be_bytes().as_ref()],
        bump = base_bid_bump,
        space = 700)]
    pub base_bid: Box<Account<'info, Bids>>,


    #[account(mut)]
    pub nft_owner: Signer<'info>,

    pub nft_mint: Account<'info, Mint>,

    #[account(mut, constraint= owners_ata.mint == nft_mint.key() )]
    pub owners_ata: Account<'info, TokenAccount>,

    #[account(init,
        payer = nft_owner,
        seeds = [auction_meta.key().as_ref()],
        bump = escrow_bump,
        token::mint = nft_mint,
        token::authority = escrowed_ata)]
    pub escrowed_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,

}

#[derive(Accounts)]
#[instruction(data_bump:u8, auction_meta_bump:u8)]
pub struct MakeBid<'info> {
    #[account( seeds = [b"data".as_ref()], bump = data_bump)]
    pub data_acc: Box<Account<'info, Data>>,

    #[account(mut,
        seeds = [nft_owner.to_account_info().key.as_ref(), nft_mint.to_account_info().key.as_ref(), auction_meta.auction_valid_till.to_be_bytes().as_ref()],
        bump
    )]
    pub auction_meta: Box<Account<'info, NFTAuction>>,

    #[account(init,
        payer = bid_maker,
        seeds = [nft_owner.to_account_info().key.as_ref(), nft_mint.to_account_info().key.as_ref(), bid_maker.to_account_info().key.as_ref(),
        auction_meta.auction_valid_till.to_be_bytes().as_ref(), (auction_meta.bids_placed + 1 as u64).to_be_bytes().as_ref()],
        bump,
        space = 700)]
    pub bid: Box<Account<'info, Bids>>,

    
    // #[account(constraint = nft_owner.key() == auction_meta.nft_owner)]
    #[account(mut)]
    pub nft_owner: AccountInfo<'info>,
    
    // #[account(constraint = nft_mint.key() == auction_meta.nft_mint)]
    #[account()]
    pub nft_mint: Account<'info, Mint>,
    
    #[account(mut)]
    pub bid_maker: Signer<'info>,
    
    #[account(mut)]
    pub previous_bidder: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [nft_owner.to_account_info().key.as_ref(), nft_mint.to_account_info().key.as_ref(),
        previous_bidder.to_account_info().key.as_ref(),
        auction_meta.auction_valid_till.to_be_bytes().as_ref(),
        (auction_meta.bids_placed as u64).to_be_bytes().as_ref()],
        bump,
        )]
    pub previous_bid: Box<Account<'info, Bids>>,


    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,

}

#[derive(Accounts)]
#[instruction(data_bump:u8, auction_meta_bump:u8, latest_bid_bump:u8, auction_valid_till:i64)]

pub struct Redeem<'info> {
    #[account( seeds = [b"data".as_ref()], bump = data_bump)]
    pub data_acc: Box<Account<'info, Data>>,

    #[account(mut,
        seeds = [nft_owner.to_account_info().key.as_ref(), nft_mint.to_account_info().key.as_ref(), auction_meta.auction_valid_till.to_be_bytes().as_ref()],
        bump
    )]
    pub auction_meta: Box<Account<'info, NFTAuction>>,

    #[account(mut ,constraint = nft_owner.key() == auction_meta.nftowner)]
    pub nft_owner: AccountInfo<'info>,
    
    #[account(constraint = nft_mint.key() == auction_meta.nftmint)]
    pub nft_mint: Account<'info, Mint>,

    pub redeemer: Signer<'info>,

    #[account(mut, 
        seeds = [auction_meta.key().as_ref()], bump = auction_meta.escrowed_nft_bump)]
    pub escrowed_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub latest_bidder: AccountInfo<'info>,

    #[account(init_if_needed, payer = redeemer, associated_token::mint = nft_mint, associated_token::authority = latest_bidder)]
    pub latest_bidders_nft_account: Box<Account<'info, TokenAccount>>,
    
    #[account(mut, constraint= owners_ata.mint == nft_mint.key() )]
    pub owners_ata: Box<Account<'info, TokenAccount>>,


    #[account(
        mut,
        seeds = [nft_owner.to_account_info().key.as_ref(), nft_mint.to_account_info().key.as_ref(),
        latest_bidder.to_account_info().key.as_ref(),
        auction_meta.auction_valid_till.to_be_bytes().as_ref(),
        (auction_meta.bids_placed as u64).to_be_bytes().as_ref()],
        bump,
        )]
    pub present_bid_acc: Box<Account<'info, Bids>>,



    #[account()]
    pub token_metadata_account: AccountInfo<'info>,
    
    #[account()]
    pub token_metadata_program: AccountInfo<'info>,

    #[account(mut)]
    pub market_maker: AccountInfo<'info>,

    #[account(mut)]
    pub creator0: AccountInfo<'info>,

    #[account(mut)]
    pub creator1: AccountInfo<'info>,

    #[account(mut)]
    pub creator2: AccountInfo<'info>,

    #[account(mut)]
    pub creator3: AccountInfo<'info>,

    #[account(mut)]
    pub creator4: AccountInfo<'info>,

    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,

} 