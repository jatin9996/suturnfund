import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { SaturnFund } from '../target/types/saturn_fund';

describe('saturn_fund', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.SaturnFund as Program<SaturnFund>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
