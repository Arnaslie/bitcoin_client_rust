// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;
import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/ISwap.sol";
import "./sAsset.sol";

contract Swap is Ownable, ISwap {
    address token0;
    address token1;
    uint reserve0;
    uint reserve1;
    mapping (address => uint) shares;
    uint public totalShares;

    constructor(address addr0, address addr1) {
        token0 = addr0;
        token1 = addr1;
    }

    function init(uint token0Amount, uint token1Amount) external override onlyOwner {
        require(reserve0 == 0 && reserve1 == 0, "init - already has liquidity");
        require(token0Amount > 0 && token1Amount > 0, "init - both tokens are needed");
        
        require(sAsset(token0).transferFrom(msg.sender, address(this), token0Amount));
        require(sAsset(token1).transferFrom(msg.sender, address(this), token1Amount));
        reserve0 = token0Amount;
        reserve1 = token1Amount;
        totalShares = sqrt(token0Amount * token1Amount);
        shares[msg.sender] = totalShares;
    }

    // https://github.com/Uniswap/v2-core/blob/v1.0.1/contracts/libraries/Math.sol
    function sqrt(uint y) internal pure returns (uint z) {
        if (y > 3) {
            z = y;
            uint x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }

    function getReserves() external view returns (uint, uint) {
        return (reserve0, reserve1);
    }

    function getTokens() external view returns (address, address) {
        return (token0, token1);
    }

    function getShares(address LP) external view returns (uint) {
        return shares[LP];
    }

    function addLiquidity(uint token0Amount) external override {
        uint token1Amount = reserve1 * token0Amount / reserve1;
        bool approved0 = ERC20(token0).approve(address(this), token0Amount);
        bool approved1 = ERC20(token1).approve(address(this), token1Amount);

        // transfer from 0
        // transfer from 1
        require(approved0, "did not approve token0 from sender");
        require(approved1, "did not approve token1 from sender");
        bool sent0 = ERC20(token0).transferFrom(msg.sender, address(this), token0Amount);
        bool sent1 = ERC20(token1).transferFrom(msg.sender, address(this), token1Amount);
        // update reserve0 & reserve1
        require(sent0, "failed to send token0 to LP");
        require(sent1, "failed to send token1 to LP");
        reserve0 += token0Amount;
        reserve1 += token1Amount;
        totalShares = sqrt(reserve0 * reserve1);    
        uint new_shares = totalShares * token0Amount / reserve0;
        // update shares map & total shares
        shares[msg.sender] += new_shares;
    }

    function removeLiquidity(uint withdrawShares) external override {
        require(shares[msg.sender] >= withdrawShares, "attempting to withdraw too many shares");
        uint amount0 = reserve0 * withdrawShares / totalShares;
        uint amount1 = reserve1 * withdrawShares / totalShares;

        // transfer from pool back to msg.sender
        bool removed0 = ERC20(token0).transfer(msg.sender, amount0);
        bool removed1 = ERC20(token1).transfer(msg.sender, amount1);
        // update reserves
        reserve0 -= amount0;
        reserve1 -= amount1;
        //update share map & total share
        shares[msg.sender] -= withdrawShares;
        totalShares = sqrt(reserve0 * reserve1);

    }

    function token0To1(uint token0Amount) external override {
        // calculate token1Amount to send to caller
        // calculate after taking into account the processing fee
        // add process fee to pool

        // make sure invariant is kept stable
        // x * y = k
        // reserve0 * reserve1 = k
        bool approved = ERC20(token0).approve(address(this), token0Amount);
        require(approved, "no approval");
        bool sent = ERC20(token0).transferFrom(msg.sender, address(this), token0Amount);
        require(sent, "failed to send tokens");

        uint invariant = reserve0 * reserve1;
        uint protocolFee = token0Amount * 3 / 1000;
        uint processedTokens = token0Amount - protocolFee;

        uint token1_to_return = reserve1 - invariant / (reserve0 + processedTokens);
        reserve0 += token0Amount;
        reserve1 -= token1_to_return;
        bool swapped = ERC20(token1).transfer(msg.sender, token1_to_return);
        require(swapped, "caller did not recieve tokens");
        // int256 diff = int256(reserve0) * int256(reserve1) - int256(invariant);
        // require(abs(diff) < 10000000000, "invariant has been unbalanced");
        /*
        ex: token0Amount = 1000
        reserve0, reserve1 = (1000000, 1000000)
        fee = 0.3%
        token1_to_return = 1000000 - 1000000 * 1000000 / (1000000 + 997) = 996
        */


    }

    function token1To0(uint token1Amount) external override {
        // same as token0to! just flipped, 
        // calculating conversion from token1 to token0
        bool approved = ERC20(token1).approve(address(this), token1Amount);
        require(approved, "no approval");
        bool sent = ERC20(token1).transferFrom(msg.sender, address(this), token1Amount);
        require(sent, "failed to send tokens");

        uint invariant = reserve0 * reserve1;
        uint protocolFee = token1Amount * 3 / 1000;
        uint processedTokens = token1Amount - protocolFee;

        uint token0_to_return = reserve0 - invariant / (reserve1 + processedTokens);
        reserve0 -= token0_to_return;
        reserve1 += token1Amount;
        bool swapped = ERC20(token0).transfer(msg.sender, token0_to_return);
        require(swapped, "caller did not recieve tokens");
        // int256 diff = int256(reserve0) * int256(reserve1) - int256(invariant);
        // require(abs(diff) < 100000000000, "invariant has been unbalanced");

    }

    function abs(int256 x) private pure returns (int256) {
        return x >= 0 ? x : -x;
    }
}
