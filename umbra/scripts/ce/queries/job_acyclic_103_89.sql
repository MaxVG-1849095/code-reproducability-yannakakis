select count(*) from imdb100, imdb68, imdb58 where imdb100.d = imdb68.s and imdb68.s = imdb58.s;