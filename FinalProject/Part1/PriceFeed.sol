// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@chainlink/contracts/src/v0.8/interfaces/AggregatorV3Interface.sol";
import "./interfaces/IPriceFeed.sol";

contract PriceFeed is IPriceFeed {
    /* TODO: implement your functions here */
    AggregatorV3Interface internal priceFeed;

    constructor() {
        // priceFeedBNB = AggregatorV3Interface(0x8993ED705cdf5e84D0a3B754b5Ee0e1783fcdF16);
        priceFeed = AggregatorV3Interface(0xb31357d152638fd1ae0853d24b9Ea81dF29E3EF2);
    }
    function getLatestPrice() public override view returns (int, uint) {
        (
            /*uint80 roundID*/,
            int price,
            /*uint startedAt*/,
            uint lastUpdatedTime,
            /*uint80 answeredInRound*/
        ) = priceFeed.latestRoundData();
        return (price, lastUpdatedTime);
    } 
}