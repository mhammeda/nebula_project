<template>
    <v-container class="align-center">
        <v-row>
            <v-col sm="12">
                <v-card elevation="0">
                    <v-card-title>
                        Showing Search Results for: {{ searchTerm }}
                    </v-card-title>
                </v-card>

                <v-row>
                    <v-col cols="8" sm="8">
                        <v-card min-height="500px">
                            <div class="overline mb-4">
                                POSTS
                            </div>
                            <v-divider></v-divider>
                            <div class="text-center mx-auto">
                                {{ postsMessage }}
                            </div>
                            <v-list-item
                                v-for="item of allPosts"
                                v-bind:key="item.id"
                            >
                                <v-list-item-content>
                                    <Post :item="item" />
                                </v-list-item-content>
                            </v-list-item>
                        </v-card>
                    </v-col>
                    <v-col cols="4" sm="4">
                        <v-card min-height="500px">
                            <div class="overline mb-4">
                                COMMUNITIES
                            </div>
                            <v-divider></v-divider>
                            <div class="text-center mx-auto">
                                {{ communitiesMessage }}
                            </div>
                            <v-list-item
                                v-for="item of allCommunities"
                                v-bind:key="item.id"
                            >
                                <v-list-item-content>
                                    <CommunityCard :item="item" />
                                </v-list-item-content>
                            </v-list-item>
                        </v-card>
                    </v-col>
                </v-row>
            </v-col>
        </v-row>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </v-container>
</template>
<script>
import Post from './posts/post.vue'
import CommunityCard from './communities/communityCard.vue'

export default {
    mounted() {
        this.searchPosts()
        this.searchCommunities()
    },
    components: {
        Post,
        CommunityCard,
    },
    data() {
        return {
            searchTerm: this.$route.params.searchTerm,
            postsMessage: '',
            communitiesMessage: '',
            allPosts: [],
            allCommunities: [],
            postSearchUrl: '/internal/posts/search',
            communitiesSearchUrl: '/internal/communities/search',
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        searchPosts() {
            const url = `${this.postSearchUrl}/${this.searchTerm}`

            this.$http
                .get(url, {}, { withCredentials: true })
                .then(response => {
                    console.log(response.data)
                    if (response.data.length > 0) {
                        const posts = response.data
                        this.postsMessage = `Showing ${response.data.length} posts`
                        this.allPosts = posts
                    } else {
                        this.postsMessage = 'No posts matched that search'
                    }
                })
                .catch(error => {
                    this.postsMessage = 'A problem occurred'
                    this.errorMessage = 'Problem contacting the server.'
                    this.errorSnackbar = true
                })
        },

        searchCommunities() {
            const url = `${this.communitiesSearchUrl}/${this.searchTerm}`

            this.$http
                .get(url, {}, { withCredentials: true })
                .then(response => {
                    console.log(response.data)
                    if (response.data.length > 0) {
                        const communities = response.data
                        this.communitiesMessage = `Showing ${response.data.length} communities`
                        this.allCommunities = communities
                    } else {
                        this.communitiesMessage =
                            'No communities matched that search'
                    }
                })
                .catch(error => {
                    this.communitiesMessage = 'failed to contact the server :('
                })
        },
    },
}
</script>
