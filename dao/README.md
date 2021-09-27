# Croncat DAO
Enabling a community to own, grow & maintain the blockchain scheduling utility.

# Mission
Provide a well balanced group of persons capable of furthering the development of Croncat, maintaining core business objectives that sustain the network & act on community improvement initiatives.

### Core Values
* Stability
* Economic Sustainability
* Community Ownership

# Are you a Croncat?
Croncat is an organism owned & maintained by dedicated people coming from a diverse set of perspectives. 
Here are the classifications of what makes up the types of people in Croncat DAO:
* **Founder** - Core contributors that provide vision, implementation & leadership
* **Agent** - The operator executing the tasks & runtime dictated by the DAO
* **Application** - Entities that need scheduled transactions
* **Commander** - Individuals that contribute to initiatives defined by the DAO, receive retainer stipend based on performance (Example: Growth Hacker)

Becoming part of the Croncat DAO is much different than token based DAOs. It requires real interaction and participation in community development, governance, research & network growth. To maintain a seat on Croncat DAO, you believe in the vision, aspire to further the mission statement, and generally provide your personal perspective to create positive outcomes for the Croncat community as a whole. Maintaining a seat on the DAO requires interactions like voting at minimum 5 times per year.

# Governance & Operations
### Council Responsibilities
DAO council will be the sole responsible entity for the development and promotion of croncat. The council will be made up of different types of persons, each bringing a unique viewpoint to the governance process to align and balance the DAO. You can think of the council representing 3 core entities: Founders, Producers, Consumers. The council will have the power to enact proposals, fund development & marketing, provide guidance on integrations and onboard further community members.

Council members are responsible to the DAO, and have a requirement to on-chain activity periodically to maintain their council seat. By staying active on the chain activities, members not only keep the community inline with their perspective, but also help create a wider base for decisions. Council members are also responsible for doing the research and diligence necessary to make sound decisions for fund allocations.

### Role Definitions

| Role | Capabilities | Definition & Perks |
|---|---|---|
|Founder|Proposal, Voting, Treasury, Core|Maintains council members, directs treasury funds towards development initiatives, maintains upgrades. Can vote on all types of proposals. Receives epoch based retainer stipend of a percentage on earned interest balance.|
|Application|Proposal, Voting|Proposes needs for application integrations, fees or similar. Can vote on operation & cost proposals. Early integration partners receive special swag & NFT.|
|Agent|Proposal, Voting|Proposes needs for agent runtime, reward amount or similar. Can vote on operation & cost proposals. Early adopters receive special swag & rare NFT.|
|Commander|Proposal|Proposes reimbursements, work initiatives, development bounties, marketing initiatives & other types of works created for furthering the growth of cron. Commanders receive differing levels of access to things like social, discussion, development session & more based on longevity and work completed.|

### Proposal Types
* **Treasury Proposal**: Items relating to or including fund or token allocation & staking accounting. 
* **Operation Cost Proposal**: Items that pay individuals for a certain finalized development or marketing initiative.
* **Custom Operation**: Special contract function calls that can include core runtime settings, interacting with other dApps or upgrading core contracts.
Council Change: Adding/Removing council members only when deemed appropriate by DAO.

### Core Operations:
Croncat core contracts have several variables that can be adjusted by the DAO to further align the needs of agents and applications. The purpose was to allow cron to not have a static economic model, but rather adjust to the runtime needs. The following variables define the name, type and intent of each parameter. Note that these settings are only allowed to be adjusted by DAO founders, but can be voted upon by members.

| Variable | Type | Description |
|---|---|---|
|paused|Boolean|In case of runtime emergency, the contract can be paused|
|owner_id|AccountId|This account represents the active DAO managing the croncat contract|
|agent_fee|Balance|The per-task fee which accrues to the agent for executing the task.|
|gas_price|Balance|This is the gas price, as set by the genesis config of near runtime. In the event that config changes, the setting can be updated to reflect that value.|
|slot_granularity|u64|The total amount of blocks to cluster into a window of execution. Example: If there are 1000 blocks and slot granularity is 100 then there will be 10 “buckets” where tasks will be slotted.|

### Core Deployment
Croncat is a living creature, developed by people and autonomously operating on the blockchain. Development will continue to be fluid, where features will be added from time to time. When a new feature is ready to be deployed, the compiled contract code will be staged on-chain, and submitted as an upgrade proposal. Core DAO members will be responsible for testing & ensuring the upgrade will not be malicious, align with all representative parties of cron DAO and meet all coding standards for production contracts. Upon successful approval of upgrade, the croncat contract will utilize a migration function to handle any/all state changes needed. In the event that there are backward incompatibilities, the DAO can decide to launch an entirely new deployed contract. This type of change will need to be communicated among all integration partnerships, publicly disclosed on social and website and maintain the legacy contract until all tasks have been completed.

# DAO Economic Governance:
The cron DAO will be responsible for appropriately allocating funds towards initiatives that benefit the whole croncat community. The following are possible incentives, each to be approved and potentially implemented by the DAO. Unless otherwise noted, each item will be available to be voted upon by all DAO members. Specific amounts are left out of this document, as they are to be proposed within the DAO.

### General Fund Management
Cron DAO will maintain two areas of funds:

1. **Treasury**: Funds allocated to treasury will contain collateral provided by tasks, accrued from staking interest, accrued from potential yield initiatives and initially seeded by cron DAO grant(s). The treasury use and allocation for all operations will be only controlled by founder level proposals and voted by founder level votes. Treasury will maintain a budget allocating the majority of funds towards operations and some towards incentives and growth. Treasury will focus on the goal of a fully self-sustaining income based on seeking revenue by accrued interest, developing features for efficiency or further revenues and maintaining the correct ratio of funds to keep ongoing tasks running.

2. **Core Operations**: These funds will be automatically managed by the croncat core contracts, and utilized for task deposits, task gas fees, overhead for upcoming task needs and agent reward payout. No funds remaining on the core contracts shall be touched by members of the croncat DAO, unless fee structures are adjusted resulting in a collateral surplus. All changes to fee structures will be actioned directly from the DAO, and surplus or other situations must be handled by cron DAO treasury.


### Treasury Collateral Uses
Core treasury collateral is made up of task fund allocation that will be used at a later time. This means that the majority of the treasury funds will not be available for spending, but rather available for use in the following revenue generation possibilities:
* **Staking**: Majority of funds will be allocated directly to whitelisted staking pools or meta staking, using a cadence based balancing mechanism to keep task funds available. 
* **Yield Initiatives**: Token farming, Liquidity Pools
* **Further possibilities**: Lending, Insurance

Not all of these items will be possible, but are mentioned here as possible DAO decisions and direction for fund allocation.


### Operations Budget
Operations will fund specific needs of the cron DAO that act like traditional business budgets. These needs will be allocated directly to individuals committed to achieving certain goals and tasks, with a set amount monthly or quarterly. These individuals are accepted by the DAO, and are re-evaluated post-acceptance after the completion of 2 calendar months to ensure funds were allocated wisely. Budget funds will be an operating expense, paid by treasury for specific outcomes:
* Retainers: Founders, Developers
* Promotion: Commanders
* Operations: Materials costs for items similar to marketing, swag, publishing, outreach lists or other DAO identified operation needs. 


### Incentives
* Bounties - Applied to hackathons & competitions
* Referral rewards - Available to any community member
* Onboarding bonuses: Early application integrations, Application fee waivers, Early agent adoption
* Ongoing bonuses: Application continued use, Agent continued support

### DAO Viral Loops
**Application Onboarding**

Applications running tasks on cron are imperative to the success of croncat. Early integrations using cron should be encouraged by a few initiatives:

1. Early integrations will reward both application and ambassador. Applications will receive a set amount of free transactions and free agent fees paid for by cron DAO. See economics section for specific reward amounts.

2. All integrations will receive a set amount of tasks that are agent fee free. Agents will still be rewarded for executing these tasks, however the amount will be paid by cron DAO.

3. Applications that have continuous tasks running for longer than 3 months or greater than 10,000 tasks will receive cross promotion on cron social and a rare NFT. If possible, the application will also be highlighted as a use case on the cron website.

**Agent Onboarding**

Agents keep the lights on for croncat and make the autonomy of cron possible. Agents will be incentivized primarily by rewards per task, but also encouraged in additional ways:

1. Promote the use of cron by onboarding new applications or tasks.

2. Refer others to become croncat agents.

3. Continuously run the croncat agent scripts for 1 year or more with minimal downtime. 

**Outreach, Community Expansion**

Cron will rely on the community of croncat commanders to grow the adoption of cron and promote awareness. Commanders will be responsible for creating network effects by the following avenues:

1. Post promotional materials for onboarding applications, agents and other commanders.

2. Produce creative pieces (video, blog, social post) that highlight and encourage cron use cases, potential functionality & more.

3. Recruit applications and agents to utilize cron. 
