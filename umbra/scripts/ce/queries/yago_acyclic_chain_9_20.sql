select count(*) from yago54 yago54_0, yago46 yago46_1, yago46 yago46_2, yago54 yago54_3, yago5, yago57 yago57_5, yago57 yago57_6, yago21, yago39 where yago54_0.d = yago46_1.d and yago46_1.s = yago46_2.s and yago46_2.d = yago54_3.d and yago54_3.s = yago5.s and yago5.d = yago57_5.s and yago57_5.d = yago57_6.d and yago57_6.s = yago21.d and yago21.s = yago39.s;