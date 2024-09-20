#packmode normal titan kappa mythic
//oil
mods.immersiveengineering.Refinery.addRecipe(<liquid:oil> * 200, <liquid:nutrient_distillation> * 100, <liquid:biodiesel> * 100, 4000);
//crusher
mods.immersiveengineering.Crusher.addRecipe(<exnihilocreatio:block_dust>, <minecraft:sand>, 2048);
//metal press
mods.immersiveengineering.MetalPress.addRecipe(<thermalfoundation:material:32>, <ore:ingotIron>, <immersiveengineering:mold:0>, 2000);
mods.immersiveengineering.MetalPress.addRecipe(<thermalfoundation:material:33>, <ore:ingotGold>, <immersiveengineering:mold:0>, 2000);
//arc furnace
mods.immersiveengineering.ArcFurnace.addRecipe(<projectred-core:resource_item:104>, <ore:ingotCopper>, <immersiveengineering:material:7>, 100, 512, [<projectred-core:resource_item:105> * 4], "Alloying");
mods.immersiveengineering.ArcFurnace.addRecipe(<projectred-core:resource_item:104>, <ore:dustCopper>, <immersiveengineering:material:7>, 100, 512, [<projectred-core:resource_item:105> * 4], "Alloying");
//embers converted recipes
mods.immersiveengineering.MetalPress.addRecipe(<embers:dust_ash>, <embers:alchemic_waste>, <embers:stamp_flat>, 3000);
mods.immersiveengineering.MetalPress.addRecipe(<thermalfoundation:material:771>, <soot:sulfur_clump>, <embers:stamp_flat>, 3000);
mods.immersiveengineering.MetalPress.addRecipe(<embers:shard_ember> * 6, <embers:crystal_ember>, <embers:stamp_flat>, 3000);
mods.immersiveengineering.MetalPress.addRecipe(<embers:plate_dawnstone>, <embers:ingot_dawnstone>, <immersiveengineering:mold:0>, 3000);