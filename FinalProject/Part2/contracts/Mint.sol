// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "./interfaces/IPriceFeed.sol";
import "./interfaces/IMint.sol";
import "./sAsset.sol";
import "./EUSD.sol";

contract Mint is Ownable, IMint{

    struct Asset {
        address token;
        uint minCollateralRatio;
        address priceFeed;
    }

    struct Position {
        uint idx;
        address owner;
        uint collateralAmount;
        address assetToken;
        uint assetAmount;
    }

    event addy(address owner);

    mapping(address => Asset) _assetMap;
    uint _currentPositionIndex;
    mapping(uint => Position) _idxPositionMap;
    address public collateralToken;
    

    constructor(address collateral) {
        collateralToken = collateral;
    }

    function registerAsset(address assetToken, uint minCollateralRatio, address priceFeed) external override onlyOwner {
        require(assetToken != address(0), "Invalid assetToken address");
        require(minCollateralRatio >= 1, "minCollateralRatio must be greater than 100%");
        require(_assetMap[assetToken].token == address(0), "Asset was already registered");
        
        _assetMap[assetToken] = Asset(assetToken, minCollateralRatio, priceFeed);
    }

    function getPosition(uint positionIndex) external view returns (address, uint, address, uint) {
        require(positionIndex < _currentPositionIndex, "Invalid index");
        Position storage position = _idxPositionMap[positionIndex];
        return (position.owner, position.collateralAmount, position.assetToken, position.assetAmount);
    }

    function getMintAmount(uint collateralAmount, address assetToken, uint collateralRatio) public view returns (uint) {
        Asset storage asset = _assetMap[assetToken];
        (int relativeAssetPrice, ) = IPriceFeed(asset.priceFeed).getLatestPrice();
        uint8 decimal = sAsset(assetToken).decimals();
        uint mintAmount = collateralAmount * (10 ** uint256(decimal)) / uint(relativeAssetPrice) / collateralRatio ;
        return mintAmount;
    }

    function checkRegistered(address assetToken) public view returns (bool) {
        return _assetMap[assetToken].token == assetToken;
    }

    /* TODO: implement your functions here */
    function openPosition(uint collateralAmount, address assetToken, uint collateralRatio) external override {
        emit addy(msg.sender);
        // require(true == false, "test");
        bool approvedCollateral = ERC20(collateralToken).approve((address(this)), collateralAmount);
        uint256 mintedTokens;
        bool sentTransfer;
        emit addy(msg.sender);
        bool sentCollateral;
        if(approvedCollateral) {
            sentCollateral = ERC20(collateralToken).transferFrom(msg.sender, address(this), collateralAmount);
        }
        if(sentCollateral) {
            if(checkRegistered(assetToken)) {  
                if(!(collateralRatio < _assetMap[assetToken].minCollateralRatio)) {
                    mintedTokens = Mint(address(this)).getMintAmount(collateralAmount, assetToken, collateralRatio);
                    sAsset(assetToken).mint(msg.sender, mintedTokens);
                    Position memory open_ = Position(_currentPositionIndex, msg.sender, collateralAmount, assetToken, mintedTokens);
                    _idxPositionMap[_currentPositionIndex] = open_;
                    _currentPositionIndex += 1;
                }
            }
        }
    }

    function closePosition(uint positionIndex) external override {
        (
            address owner, 
            uint collateralAmount, 
            address assetToken, 
            uint assetAmount
        ) = Mint(address(this)).getPosition(positionIndex);
        // transfer sAsset tokens from sender to contract and burn the tokens
        require(owner == msg.sender, "msg sender does not equal the owner of the position");

        // bool assetFromSender = ERC20(assetToken).approve(address(this), assetAmount);
        bool returnTokens;
        uint8 decimals = sAsset(assetToken).decimals();
        sAsset(assetToken).burn(msg.sender, assetAmount);

        // sending back collateral locked in the position
        bool returnCollateral = ERC20(collateralToken).transfer(msg.sender, collateralAmount);

        // delete the position
        if(returnCollateral) {
            delete _idxPositionMap[positionIndex];
        }
    }

    function deposit(uint positionIndex, uint collateralAmount) external override {
       (
           address owner, 
           uint curr_collateralAmount,
           address assetToken,
           uint assetAmount
       ) = Mint(address(this)).getPosition(positionIndex);
       require(owner == msg.sender, "invalid owner address");
       uint tokens = collateralAmount;
       bool depositTokens = ERC20(collateralToken).approve(address(this), tokens);
       bool deposited;
       if(depositTokens) {
           deposited = ERC20(collateralToken).transferFrom(msg.sender, address(this), tokens);
       }
       if(deposited) {
           _idxPositionMap[positionIndex].collateralAmount += tokens;
       }

    }

    function withdraw(uint positionIndex, uint withdrawAmount) external override {
        (
           address owner, 
           uint collateralAmount,
           address assetToken,
           uint assetAmount
       ) = Mint(address(this)).getPosition(positionIndex);
       bool withdrawn;
       require(owner == msg.sender, "invalid owner address");
       uint reducedCollateral = collateralAmount - withdrawAmount;
       uint reducedMCR = reducedCollateral / assetAmount;
       if(reducedMCR < _assetMap[assetToken].minCollateralRatio) {
           return;
       }
       else {
           withdrawn = ERC20(collateralToken).transfer(msg.sender, withdrawAmount);
       }
       if(withdrawn) {
           _idxPositionMap[positionIndex].collateralAmount -= withdrawAmount;
       }
    }

    function mint(uint positionIndex, uint mintAmount) external override {
        address owner = _idxPositionMap[positionIndex].owner;
        require(owner == msg.sender, "invalid owner address");

        uint collateral = _idxPositionMap[positionIndex].collateralAmount;
        address assetToken_ = _idxPositionMap[positionIndex].assetToken;
        uint assetAmount_ = _idxPositionMap[positionIndex].assetAmount;

        uint8 decimals = sAsset(assetToken_).decimals();
        uint minted = mintAmount * 10 ** decimals;

        uint increasedAssetAmount = assetAmount_ + mintAmount;
        // check new token above remains above asset MCR
        if(!(_idxPositionMap[positionIndex].collateralAmount / increasedAssetAmount < _assetMap[assetToken_].minCollateralRatio)) {
            sAsset(assetToken_).mint(msg.sender, mintAmount);
            _idxPositionMap[positionIndex].assetAmount += mintAmount;
        }
    }

    function burn(uint positionIndex, uint burnAmount) external override {
        address owner = _idxPositionMap[positionIndex].owner;
        require(owner == msg.sender, "invalid owner address");

        uint collateral = _idxPositionMap[positionIndex].collateralAmount;
        address assetToken_ = _idxPositionMap[positionIndex].assetToken;
        uint assetAmount_ = _idxPositionMap[positionIndex].assetAmount;

        uint8 decimals = sAsset(assetToken_).decimals();
        uint burned = burnAmount * 10 ** decimals;

        sAsset(assetToken_).burn(msg.sender, burnAmount);
        _idxPositionMap[positionIndex].assetAmount -= burnAmount;
    }





    


}


// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./interfaces/IPriceFeed.sol";
import "./interfaces/IMint.sol";
import "./sAsset.sol";
import "./EUSD.sol";

contract Mint is Ownable, IMint{
    struct Asset {
        address token;
        uint minCollateralRatio;
        address priceFeed;
    }

    struct Position {
        uint idx;
        address owner;
        uint collateralAmount;
        address assetToken;
        uint assetAmount;
    }

    mapping(address => Asset) _assetMap;
    uint _currentPositionIndex;
    mapping(uint => Position) _idxPositionMap;
    address public collateralToken;

    event debug();
    
    constructor(address collateral) {
        collateralToken = collateral;
    }

    function registerAsset(address assetToken, uint minCollateralRatio, address priceFeed) external override onlyOwner {
        require(assetToken != address(0), "Invalid assetToken address");
        require(minCollateralRatio >= 1, "minCollateralRatio must be greater than 100%");
        require(_assetMap[assetToken].token == address(0), "Asset was already registered");
        
        _assetMap[assetToken] = Asset(assetToken, minCollateralRatio, priceFeed);
    }

    function getPosition(uint positionIndex) external view returns (address, uint, address, uint) {
        require(positionIndex < _currentPositionIndex, "Invalid index");
        Position storage position = _idxPositionMap[positionIndex];
        return (position.owner, position.collateralAmount, position.assetToken, position.assetAmount);
    }

    function getMintAmount(uint collateralAmount, address assetToken, uint collateralRatio) public view returns (uint) {
        Asset storage asset = _assetMap[assetToken];
        (int relativeAssetPrice, ) = IPriceFeed(asset.priceFeed).getLatestPrice();
        uint8 decimal = sAsset(assetToken).decimals();
        //assetAmount = collateralAmount * (10 ** uint256(decimal)) / (uint(relativeAssetPrice) * collateralRatio)
        uint mintAmount = collateralAmount * (10 ** uint256(decimal)) / uint(relativeAssetPrice) / collateralRatio ;
        return mintAmount;
    }

    function checkRegistered(address assetToken) public view returns (bool) {
        return _assetMap[assetToken].token == assetToken;
    }

    function openPosition(uint collateralAmount, address assetToken, uint collateralRatio) external override {
        require(checkRegistered(assetToken), "openPosition: asset not registered!");
        require(collateralRatio >= _assetMap[assetToken].minCollateralRatio, "openPosition: collateralRatio is less than minimum ratio required!");

        //collateral contract approves this contract to hold collateralAmount
        bool approved = ERC20(collateralToken).approve(address(this), collateralAmount);
        require(approved, "openPosition: collateralToken didn't approve collateral amount!");

        //send collateral from sender to this contract; for this project think of the sender as the test environment.
        //think of it like this: when we initialize EUSD in test environemnt, the test environment gets some amount from deploying EUSD.
        //msg.sender is the test environment so balanceOf(msg.sender) == balanceOf(testEnv)
        bool collateralSent = ERC20(collateralToken).transferFrom(msg.sender, address(this), collateralAmount);
        require(collateralSent, "openPosition: collateral was not sent!");

        //got collateral, time to mint for sender; need to mint using asset's contract
        uint assetAmount = Mint(address(this)).getMintAmount(collateralAmount, assetToken, collateralRatio);
        sAsset(assetToken).mint(msg.sender, assetAmount);
        _idxPositionMap[_currentPositionIndex] = Position(_currentPositionIndex, msg.sender, collateralAmount, assetToken, assetAmount);
        _currentPositionIndex += 1;
    }

    function closePosition(uint positionIndex) external  override {
        (
            address owner,
            uint collateralAmount,
            address assetToken,
            uint assetAmount
        ) = Mint(address(this)).getPosition(positionIndex);

        require(msg.sender == owner, "closePosition: message sender is not owner of the position!");

        //burn asset tokens
        sAsset(assetToken).burn(msg.sender, assetAmount);

        //transfer collateral tokens backs to sender
        bool transferred = ERC20(collateralToken).transfer(msg.sender, collateralAmount);
        require(transferred, "closePosition: collateralAmount not transferred back to owner!");

        //close position locally -> delete sets all values to 0
        delete _idxPositionMap[positionIndex];
    }

    function deposit(uint positionIndex, uint collateralAmount) external override  {
        (
            address owner,
            uint oldCollateralAmount,
            address assetToken,
            uint assetAmount
        ) = Mint(address(this)).getPosition(positionIndex);

        require(msg.sender == owner, "deposit: message sender is not owner of the position!");

        uint newCollateralAmount = oldCollateralAmount + collateralAmount;

        //update collateral contract to approve new collateral amount for this contract
        bool approved = ERC20(collateralToken).approve(address(this), newCollateralAmount);
        require(approved, "deposit: collateralToken didn't approve new collateral amount!");

        //send additional collateral from sender to this contract; for this project think of the sender as the test environment.
        //think of it like this: when we initialize EUSD in test environemnt, the test environment gets some amount from deploying EUSD.
        //msg.sender is the test environment so balanceOf(msg.sender) == balanceOf(testEnv)
        bool collateralSent = ERC20(collateralToken).transferFrom(msg.sender, address(this), collateralAmount);
        require(collateralSent, "deposit: collateral was not sent!");

        //update collateral for position
        _idxPositionMap[positionIndex].collateralAmount = newCollateralAmount;
    }

    function withdraw(uint positionIndex, uint withdrawAmount) external override  {
        (
            address owner,
            uint collateralAmount,
            address assetToken,
            uint assetAmount
        ) = Mint(address(this)).getPosition(positionIndex);

        require(msg.sender == owner, "withdraw: message sender is not owner of the position!");

        require(collateralAmount >= withdrawAmount, "withdraw: cannot withdraw more collateral than being held!");
        uint newCollateralAmount = collateralAmount - withdrawAmount;

        //update collateral contract to approve new collateral amount for this contract
        bool approved = ERC20(collateralToken).approve(address(this), newCollateralAmount);
        require(approved, "withdraw: collateralToken didn't approve new collateral amount!");

        //collateral ratio = collateral value / asset value = (collateralAmount * collateral token value) / (assetAmount * asset token price);
        //in this project, the prices of BOTH the collateral token AND the asset token is WITH RESPECT TO the collateral token value;
        //thus, collateral token value becomes 1 (ratio with respect to itself) and asset token price is the value from the price feed
        //contract / 10^decimals (it's hardcoded: value in USD * 10**decimal)
        (int relativeAssetPrice, ) = IPriceFeed(_assetMap[assetToken].priceFeed).getLatestPrice();
        uint8 decimal = sAsset(assetToken).decimals();
        uint newCollateralRatio = newCollateralAmount / (assetAmount * (uint(relativeAssetPrice) / (10 ** uint256(decimal))));
        require(newCollateralRatio >= _assetMap[assetToken].minCollateralRatio, "withdraw: new collateral ratio is below the minimum required!");

        bool withdrewCollateral = ERC20(collateralToken).transfer(msg.sender, withdrawAmount);
        require(withdrewCollateral, "withdraw: collateral not transferred back to sender!");

        //update collateral for position
        _idxPositionMap[positionIndex].collateralAmount = newCollateralAmount;
    }

    function mint(uint positionIndex, uint mintAmount) external override  {
        (
            address owner,
            uint collateralAmount,
            address assetToken,
            uint assetAmount
        ) = Mint(address(this)).getPosition(positionIndex);

        require(msg.sender == owner, "mint: message sender is not owner of the position!");

        uint newAssetAmount = assetAmount + mintAmount;

        //collateral ratio = collateral value / asset value = (collateralAmount * collateral token value) / (assetAmount * asset token price);
        //in this project, the prices of BOTH the collateral token AND the asset token is WITH RESPECT TO the collateral token value;
        //thus, collateral token value becomes 1 (ratio with respect to itself) and asset token price is the value from the price feed
        //contract / 10^decimals (it's hardcoded: value in USD * 10**decimal)
        (int relativeAssetPrice, ) = IPriceFeed(_assetMap[assetToken].priceFeed).getLatestPrice();
        uint8 decimal = sAsset(assetToken).decimals();
        uint newCollateralRatio = collateralAmount / (newAssetAmount * (uint(relativeAssetPrice) / (10 ** uint256(decimal))));
        require(newCollateralRatio >= _assetMap[assetToken].minCollateralRatio, "mint: new collateral ratio is below the minimum required!");

        //mint sender's tokens in asset contract and update locally
        sAsset(assetToken).mint(msg.sender, mintAmount);
        _idxPositionMap[positionIndex].assetAmount = newAssetAmount;
    }

    function burn(uint positionIndex, uint burnAmount) external override  {
        (
            address owner,
            uint collateralAmount,
            address assetToken,
            uint assetAmount
        ) = Mint(address(this)).getPosition(positionIndex);

        require(msg.sender == owner, "burn: message sender is not owner of the position!");

        require(assetAmount >= burnAmount, "burn: cannot burn more tokens than being held!");
        uint newAssetAmount = assetAmount - burnAmount;

        //burn sender's tokens in asset contract and update locally
        sAsset(assetToken).burn(msg.sender, burnAmount);
        _idxPositionMap[positionIndex].assetAmount = newAssetAmount;
    }
}
