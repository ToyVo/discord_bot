#packmode normal titan kappa mythic
//TE induction smelter
mods.thermalexpansion.InductionSmelter.removeRecipe(<minecraft:sand:*>, <minecraft:compass>);
mods.thermalexpansion.InductionSmelter.removeRecipe(<minecraft:sand:*>, <minecraft:clock>);
//TE pulverizer
mods.thermalexpansion.Pulverizer.addRecipe(<exnihilocreatio:block_dust>, <minecraft:sand>, 1500, <exnihilocreatio:block_dust>, 5);
mods.thermalexpansion.Pulverizer.addRecipe(<astralsorcery:itemcraftingcomponent:0> * 6, <contenttweaker:aquamarine_block>, 3600);
//TE redstone furnace
mods.thermalexpansion.RedstoneFurnace.addRecipe(<astralsorcery:itemcraftingcomponent:0> * 3, <contenttweaker:aquamarine_block>, 3600);
//TE transposer
mods.thermalexpansion.Transposer.addFillRecipe(<contenttweaker:neutral_steel_1>, <draconicevolution:draconium_ingot>, <liquid:pyrotheum> * 2000, 5000);
mods.thermalexpansion.Transposer.addFillRecipe(<contenttweaker:neutral_steel_2>, <contenttweaker:neutral_steel_1> * 3, <liquid:cryotheum> * 2000, 5000);
mods.thermalexpansion.Transposer.addFillRecipe(<contenttweaker:neutral_steel_3>, <contenttweaker:neutral_steel_2> * 3, <liquid:aerotheum> * 2000, 5000);
mods.thermalexpansion.Transposer.addFillRecipe(<contenttweaker:neutral_steel_4>, <contenttweaker:neutral_steel_3> * 3, <liquid:petrotheum> * 2000, 5000);
//TE phytonic insolator
mods.thermalexpansion.Insolator.removeRecipe(<appliedenergistics2:crystal_seed:1200>, <minecraft:glowstone_dust>);
mods.thermalexpansion.Insolator.removeRecipe(<appliedenergistics2:crystal_seed:600>, <minecraft:glowstone_dust>);
mods.thermalexpansion.Insolator.removeRecipe(<appliedenergistics2:crystal_seed:0>, <minecraft:glowstone_dust>);