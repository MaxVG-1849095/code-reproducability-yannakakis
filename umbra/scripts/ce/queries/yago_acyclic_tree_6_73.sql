select count(*) from yago62, yago21 yago21_1, yago21 yago21_2, yago21 yago21_3, yago23, yago5 where yago62.s = yago21_1.d and yago21_1.d = yago21_2.d and yago21_2.s = yago21_3.s and yago21_3.s = yago23.d and yago21_3.d = yago5.d;