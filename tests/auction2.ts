import * as anchor from '@project-serum/anchor';
import * as spl from '@solana/spl-token';
import { Program } from '@project-serum/anchor';
import { Auction2 } from '../target/types/auction2';

describe('auction2', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Auction2 as Program<Auction2>;

  it('Is initialized!', async () => {
    // Add your test here.
    console.log(program.programId);
    console.log(program.provider.wallet);
  
    const [data, data_bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("data")], program.programId);
    const NFTMintKeyPair = anchor.web3.Keypair.generate();
    const NFTMintPubKey = NFTMintKeyPair.publicKey;
    //const NFTOwnerKeyPair = anchor.web3.Keypair.generate();
    const NFTOwnerPubKey = program.provider.wallet.publicKey;
    const bf = anchor.web3.Keypair.generate();
    const bfPubKey = bf.publicKey;
    const pdaRent = anchor.web3.Keypair.generate();
    const pdaRentPubKey = pdaRent.publicKey;

    const [auctionmeta, auctionmetabump] = await anchor.web3.PublicKey.findProgramAddress([NFTOwnerPubKey.toBuffer(), NFTMintPubKey.toBuffer()], program.programId);
    console.log("PUBKEY", program.provider.wallet.publicKey.toString());
    console.log("AUCTION META is", auctionmeta.toString());
    console.log("AUCTION META BUMP is", auctionmetabump);
    const mkCut = new anchor.BN(25);
    let tx = await program.rpc.new(data_bump, mkCut, { accounts:{
      dataAcc: data,
      payer: program.provider.wallet.publicKey,
      beneficiary:  bfPubKey, 
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      pdaRent: pdaRentPubKey,
    }});
    console.log("Your transaction signature", tx);

    while ((await program.provider.connection.getSignatureStatus(tx)).value.confirmations === 0) {
    
    }

    console.log("STARTING AUCTION");
    tx = await program.rpc.startAuction(data_bump, auctionmetabump, { accounts:{
      dataAcc: data,
      auctionMeta: auctionmeta,
      nftOwner: program.provider.wallet.publicKey,
      nftMint: NFTMintPubKey,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    }});
    //console.log(program);
    //START AUCTION
    console.log("FETCHING AUCTION META");
    let auctionmetaacc = (await program.account.nftAuction.fetch(auctionmeta));
    console.log("FETCHED ACUTION META ACC IS", auctionmetaacc.toString());
    console.log("AUCT META2 BIDS MADE is", auctionmetaacc.bidPrice.toString());
    console.log("AUCT META BIDS PLACED2 is", auctionmetaacc.bidsPlaced.toString());
 
  });

});
