select count(*) from imdb1, imdb122, imdb3, imdb88 where imdb1.s = imdb122.s and imdb122.d = imdb3.d and imdb3.d = imdb88.s;